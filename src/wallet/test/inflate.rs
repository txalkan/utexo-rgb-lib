use super::*;

#[cfg(feature = "electrum")]
#[test]
#[parallel]
fn success() {
    initialize();

    // wallets
    let (mut wallet, online) = get_funded_wallet!();
    let (mut rcv_wallet, rcv_online) = get_funded_wallet!();

    // create more UTXOs
    let _ = test_create_utxos_default(&mut wallet, &online);

    // issue
    let issue_amounts = [AMOUNT, AMOUNT];
    let inflation_rights = [100, 500, 200];
    let asset = test_issue_asset_ifa(
        &mut wallet,
        &online,
        Some(&issue_amounts),
        Some(&inflation_rights),
        0,
        None,
    );
    let initial_supply = issue_amounts.iter().sum::<u64>();
    let max_supply = initial_supply + inflation_rights.iter().sum::<u64>();
    assert_eq!(asset.initial_supply, initial_supply);
    assert_eq!(asset.known_circulating_supply, initial_supply);
    assert_eq!(asset.max_supply, max_supply);
    let transfers = test_list_transfers(&wallet, Some(&asset.asset_id));
    assert_eq!(transfers.len(), 1);
    assert_eq!(transfers.first().unwrap().kind, TransferKind::Issuance);

    // inflate
    let inflation_amounts = [199, 42];
    let res = test_inflate(&mut wallet, &online, &asset.asset_id, &inflation_amounts);
    let total_inflated = inflation_amounts.iter().sum::<u64>();

    mine(false, false);

    assert!(test_refresh_asset(&mut wallet, &online, &asset.asset_id));

    let transfers = test_list_transfers(&wallet, Some(&asset.asset_id));
    assert_eq!(transfers.len(), 3);
    let mut created_at = None;
    let mut updated_at = None;
    let mut recipient_id = None;
    let mut receive_utxo = None;
    let mut change_utxo = None;
    let mut expiration = None;
    let mut inflation_rights_sorted = inflation_rights;
    inflation_rights_sorted.sort();
    let inflation_change = inflation_rights_sorted.iter().take(2).sum::<u64>() - total_inflated;
    for (i, amt) in inflation_amounts.iter().enumerate() {
        let transfer = transfers.get(i + 1).unwrap();
        assert_eq!(transfer.batch_transfer_idx, 2);
        if created_at.is_none() {
            created_at = Some(transfer.created_at);
        } else {
            assert_eq!(created_at, Some(transfer.created_at));
        }
        if updated_at.is_none() {
            updated_at = Some(transfer.updated_at);
        } else {
            assert_eq!(updated_at, Some(transfer.updated_at));
        }
        assert_eq!(transfer.status, TransferStatus::Settled);
        assert_eq!(
            transfer.requested_assignment.as_ref().unwrap(),
            &Assignment::Fungible(*amt)
        );
        assert_eq!(
            transfer.assignments,
            vec![
                Assignment::InflationRight(inflation_change),
                Assignment::Fungible(inflation_amounts[0]),
                Assignment::Fungible(inflation_amounts[1])
            ]
        );
        assert_eq!(transfer.kind, TransferKind::Inflation);
        assert_eq!(transfer.txid.as_ref().unwrap(), &res.txid);
        assert!(transfer.recipient_id.is_some());
        if recipient_id.is_none() {
            recipient_id = transfer.recipient_id.clone();
        } else {
            assert_ne!(recipient_id, transfer.recipient_id)
        }
        assert!(transfer.receive_utxo.is_some());
        if receive_utxo.is_none() {
            receive_utxo = transfer.receive_utxo.clone();
        } else {
            assert_ne!(receive_utxo, transfer.receive_utxo)
        }
        assert!(transfer.change_utxo.is_some());
        if change_utxo.is_none() {
            change_utxo = transfer.change_utxo.clone();
        } else {
            assert_eq!(change_utxo, transfer.change_utxo)
        }
        assert!(transfer.expiration.is_some());
        if expiration.is_none() {
            expiration = transfer.expiration;
        } else {
            assert_eq!(expiration, transfer.expiration)
        }
        assert!(transfer.transport_endpoints.is_empty());
        assert!(transfer.invoice_string.is_none());
        assert!(transfer.consignment_path.is_some());
    }

    let total_issued = initial_supply + total_inflated;
    let balance = test_get_asset_balance(&wallet, &asset.asset_id);
    assert_eq!(
        balance,
        Balance {
            settled: total_issued,
            future: total_issued,
            spendable: total_issued,
        }
    );

    // send
    let receive_data = test_blind_receive(&rcv_wallet);
    let recipient_map = HashMap::from([(
        asset.asset_id.clone(),
        vec![Recipient {
            assignment: Assignment::Fungible(total_issued),
            recipient_id: receive_data.recipient_id.clone(),
            witness_data: None,
            transport_endpoints: TRANSPORT_ENDPOINTS.clone(),
        }],
    )]);
    let bak_info_before = wallet.database.get_backup_info().unwrap().unwrap();
    let txid = test_send(&mut wallet, &online, &recipient_map);
    let bak_info_after = wallet.database.get_backup_info().unwrap().unwrap();
    assert!(bak_info_after.last_operation_timestamp > bak_info_before.last_operation_timestamp);
    assert!(!txid.is_empty());
    let (transfer, _, _) = get_test_transfer_sender(&wallet, &txid);
    let tte_data = wallet
        .database
        .get_transfer_transport_endpoints_data(transfer.idx)
        .unwrap();
    assert_eq!(tte_data.len(), 1);
    let ce = tte_data.first().unwrap();
    assert_eq!(ce.1.endpoint, PROXY_URL);
    assert!(ce.0.used);

    // transfers progress to status WaitingConfirmations after a refresh
    std::thread::sleep(Duration::from_millis(1000)); // make sure updated_at will be at least +1s
    wait_for_refresh(&mut rcv_wallet, &rcv_online, None, None);
    let rcv_transfer = get_test_transfer_recipient(&rcv_wallet, &receive_data.recipient_id);
    let (rcv_transfer_data, _rcv_asset_transfer) =
        get_test_transfer_data(&rcv_wallet, &rcv_transfer);
    wait_for_refresh(&mut wallet, &online, Some(&asset.asset_id), None);
    let (transfer, _, _) = get_test_transfer_sender(&wallet, &txid);
    let (transfer_data, _) = get_test_transfer_data(&wallet, &transfer);
    assert_eq!(
        rcv_transfer_data.status,
        TransferStatus::WaitingConfirmations
    );
    assert_eq!(transfer_data.status, TransferStatus::WaitingConfirmations);

    // asset has been received correctly
    let rcv_assets = test_list_assets(&rcv_wallet, &[]);
    let ifa_assets = rcv_assets.ifa.unwrap();
    assert_eq!(ifa_assets.len(), 1);
    let rcv_asset = ifa_assets.last().unwrap();
    assert_eq!(rcv_asset.asset_id, asset.asset_id);
    assert_eq!(rcv_asset.ticker, TICKER);
    assert_eq!(rcv_asset.name, NAME);
    assert_eq!(rcv_asset.precision, PRECISION);
    assert_eq!(
        rcv_asset.balance,
        Balance {
            settled: 0,
            future: total_issued,
            spendable: 0,
        }
    );
    assert_eq!(rcv_asset.initial_supply, initial_supply);
    assert_eq!(rcv_asset.max_supply, max_supply);
    assert_eq!(
        rcv_asset.known_circulating_supply,
        initial_supply + total_inflated
    );

    // transfers progress to status Settled after tx mining + refresh
    mine(false, false);
    std::thread::sleep(Duration::from_millis(1000)); // make sure updated_at will be at least +1s
    wait_for_refresh(&mut rcv_wallet, &rcv_online, None, None);
    wait_for_refresh(&mut wallet, &online, Some(&asset.asset_id), None);

    let rcv_transfer = get_test_transfer_recipient(&rcv_wallet, &receive_data.recipient_id);
    let (rcv_transfer_data, _) = get_test_transfer_data(&rcv_wallet, &rcv_transfer);
    let (transfer, _, _) = get_test_transfer_sender(&wallet, &txid);
    let (transfer_data, _) = get_test_transfer_data(&wallet, &transfer);
    assert_eq!(rcv_transfer_data.status, TransferStatus::Settled);
    assert_eq!(transfer_data.status, TransferStatus::Settled);
    assert_eq!(transfer_data.change_utxo, None);

    let asset_metadata = test_get_asset_metadata(&rcv_wallet, &asset.asset_id);
    assert_eq!(asset_metadata.initial_supply, initial_supply);
    assert_eq!(asset_metadata.max_supply, max_supply);
    assert_eq!(
        asset_metadata.known_circulating_supply,
        initial_supply + total_inflated
    );
}

#[cfg(feature = "electrum")]
#[test]
#[parallel]
fn fail() {
    initialize();

    let (mut wallet, online) = get_funded_wallet!();

    // unsupported inflation
    let asset_nia = test_issue_asset_nia(&mut wallet, &online, None);
    let asset_cfa = test_issue_asset_cfa(&mut wallet, &online, None, None);
    let asset_uda = test_issue_asset_uda(&mut wallet, &online, None, None, vec![]);
    let unsupported_asset_ids = [
        (asset_nia.asset_id, AssetSchema::Nia),
        (asset_cfa.asset_id, AssetSchema::Cfa),
        (asset_uda.asset_id, AssetSchema::Uda),
    ];
    for (asset_id, schema) in unsupported_asset_ids {
        let inflation_amounts = vec![200, 42];
        let result = wallet.inflate(
            online.clone(),
            asset_id,
            inflation_amounts,
            FEE_RATE,
            MIN_CONFIRMATIONS,
        );
        assert_matches!(result, Err(Error::UnsupportedInflation { asset_schema }) if asset_schema == schema);
    }
}
