use crate::ext;
use crate::mock::*;
use crate::PolkaBTC;
use crate::Replace as R;
use crate::DOT;
use bitcoin::types::H256Le;
use frame_support::assert_noop;
use mocktopus::mocking::*;
use primitive_types::H256;
use sp_core::H160;
use vault_registry::Vault;
use x_core::Error;

fn test_request() -> R<u64, u64, u64, u64> {
    R {
        new_vault: None,
        old_vault: ALICE,
        open_time: 0,
        accept_time: None,
        amount: 10,
        griefing_collateral: 0,
        btc_address: H160([0; 20]),
        collateral: 20,
    }
}

fn test_vault() -> Vault<u64, u64, u64> {
    Vault {
        id: BOB,
        banned_until: None,
        issued_tokens: 5,
        btc_address: H160([0; 20]),
        to_be_issued_tokens: 0,
        to_be_redeemed_tokens: 0,
    }
}

fn request_replace(
    origin: AccountId,
    vault: AccountId,
    amount: Balance,
    timeout: BlockNumber,
    griefing_collateral: DOT<Test>,
) -> Result<H256, Error> {
    Replace::_request_replace(origin, vault, amount, timeout, griefing_collateral)
}

fn withdraw_replace(vault_id: AccountId, replace_id: H256) -> Result<(), Error> {
    Replace::_withdraw_replace_request(vault_id, replace_id)
}

fn accept_replace(
    vault_id: AccountId,
    replace_id: H256,
    collateral: DOT<Test>,
) -> Result<(), Error> {
    Replace::_accept_replace(vault_id, replace_id, collateral)
}

fn auction_replace(
    old_vault_id: AccountId,
    new_vault_id: AccountId,
    btc_amount: PolkaBTC<Test>,
    collateral: DOT<Test>,
) -> Result<(), Error> {
    Replace::_auction_replace(old_vault_id, new_vault_id, btc_amount, collateral)
}

fn execute_replace(
    new_vault_id: AccountId,
    replace_id: H256,
    tx_id: H256Le,
    tx_block_height: u32,
    merkle_proof: Vec<u8>,
    raw_tx: Vec<u8>,
) -> Result<(), Error> {
    Replace::_execute_replace(
        new_vault_id,
        replace_id,
        tx_id,
        tx_block_height,
        merkle_proof,
        raw_tx,
    )
}

fn cancel_replace(new_vault_id: AccountId, replace_id: H256) -> Result<(), Error> {
    Replace::_cancel_replace(new_vault_id, replace_id)
}

#[test]
fn test_request_replace_transfer_zero_fails() {
    run_test(|| {
        assert_noop!(request_replace(0, BOB, 0, 0, 0), Error::InvalidAmount);
    })
}

#[test]
fn test_request_replace_timeout_zero_fails() {
    run_test(|| {
        assert_noop!(request_replace(0, BOB, 1, 0, 0), Error::InvalidTimeout);
    })
}

#[test]
fn test_request_replace_vault_not_found_fails() {
    run_test(|| {
        assert_noop!(request_replace(0, 10_000, 1, 1, 0), Error::VaultNotFound);
    })
}

#[test]
fn test_request_replace_vault_banned_fails() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 0,
                to_be_redeemed_tokens: 0,
                btc_address: H160([0; 20]),
                banned_until: Some(1),
            }))
        });
        assert_noop!(
            Replace::_request_replace(ALICE, BOB, 1, 1, 0),
            Error::VaultBanned
        );
    })
}

#[test]
fn test_request_replace_insufficient_collateral_fails() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_| {
            MockResult::Return(Ok(Vault {
                id: BOB,
                to_be_issued_tokens: 0,
                issued_tokens: 10,
                to_be_redeemed_tokens: 0,
                btc_address: H160([0; 20]),
                banned_until: None,
            }))
        });
        ext::vault_registry::is_over_minimum_collateral::<Test>
            .mock_safe(|_| MockResult::Return(true));
        ext::collateral::get_collateral_from_account::<Test>.mock_safe(|_| MockResult::Return(1));
        assert_noop!(
            Replace::_request_replace(ALICE, BOB, 1, 1, 0),
            Error::InsufficientCollateral
        );
    })
}

#[test]
fn test_withdraw_replace_request_invalid_replace_id_fails() {
    run_test(|| {
        Replace::get_replace_request
            .mock_safe(|_| MockResult::Return(Err(Error::InvalidReplaceID)));
        assert_noop!(
            Replace::_withdraw_replace_request(ALICE, H256([0u8; 32])),
            Error::InvalidReplaceID
        );
    })
}

#[test]
fn test_withdraw_replace_request_invalid_vault_id_fails() {
    run_test(|| {
        Replace::get_replace_request.mock_safe(|_| MockResult::Return(Ok(test_request())));
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Err(Error::VaultNotFound)));
        assert_noop!(
            withdraw_replace(ALICE, H256([0u8; 32])),
            Error::VaultNotFound
        );
    })
}

#[test]
fn test_withdraw_replace_req_vault_id_mismatch_fails() {
    run_test(|| {
        Replace::get_replace_request.mock_safe(|_| MockResult::Return(Ok(test_request())));
        // TODO(jaupe): work out why this is not mocking correctly
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_id| MockResult::Return(Ok(test_vault())));
        assert_noop!(
            withdraw_replace(BOB, H256([0u8; 32])),
            Error::UnauthorizedVault
        );
    })
}

#[test]
fn test_withdraw_replace_req_under_secure_threshold_fails() {
    run_test(|| {
        Replace::get_replace_request.mock_safe(|_| MockResult::Return(Ok(test_request())));
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_id| {
            MockResult::Return(Ok({
                let mut v = test_vault();
                v.id = ALICE;
                v
            }))
        });
        ext::vault_registry::is_collateral_below_secure_threshold::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(true)));
        ext::collateral::get_collateral_from_account::<Test>.mock_safe(|_| MockResult::Return(0));
        assert_noop!(
            withdraw_replace(BOB, H256([0u8; 32])),
            Error::UnauthorizedVault
        );
    })
}

#[test]
fn test_withdraw_replace_req_has_new_owner_fails() {
    run_test(|| {
        Replace::get_replace_request.mock_safe(|_| {
            let mut r = test_request();
            r.old_vault = ALICE;
            r.new_vault = Some(3);
            MockResult::Return(Ok(r))
        });
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_id| {
            MockResult::Return(Ok({
                let mut v = test_vault();
                v.id = ALICE;
                v
            }))
        });
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(false)));
        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(20_000_000));
        assert_noop!(
            withdraw_replace(ALICE, H256([0u8; 32])),
            Error::CancelAcceptedRequest
        );
    })
}

#[test]
fn test_accept_replace_bad_replace_id_fails() {
    run_test(|| {
        Replace::get_replace_request.mock_safe(|_| {
            let mut r = test_request();
            r.old_vault = ALICE;
            r.new_vault = Some(3);
            MockResult::Return(Err(Error::InvalidReplaceID))
        });
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_id| {
            MockResult::Return(Ok({
                let mut v = test_vault();
                v.id = ALICE;
                v
            }))
        });
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(true)));
        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(20_000_000));
        let collateral = 100_000;
        assert_noop!(
            accept_replace(ALICE, H256([0u8; 32]), collateral),
            Error::InvalidReplaceID
        );
    })
}

#[test]
fn test_accept_replace_bad_vault_id_fails() {
    run_test(|| {
        Replace::get_replace_request.mock_safe(|_| {
            let mut r = test_request();
            r.old_vault = ALICE;
            r.new_vault = Some(3);
            MockResult::Return(Ok(r))
        });
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_id| MockResult::Return(Err(Error::InvalidVaultID)));
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(false)));
        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(20_000_000));
        let collateral = 100_000;
        assert_noop!(
            accept_replace(ALICE, H256([0u8; 32]), collateral),
            Error::InvalidVaultID
        );
    })
}

#[test]
fn test_accept_replace_vault_banned_fails() {
    run_test(|| {
        Replace::get_replace_request.mock_safe(|_| {
            let mut r = test_request();
            r.old_vault = ALICE;
            r.new_vault = Some(3);
            MockResult::Return(Ok(r))
        });
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_id| {
            let mut vault = test_vault();
            vault.banned_until = Some(100);
            MockResult::Return(Ok(vault))
        });
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(false)));
        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(20_000_000));
        let collateral = 100_000;
        assert_noop!(
            accept_replace(ALICE, H256([0u8; 32]), collateral),
            Error::VaultBanned
        );
    })
}

#[test]
fn test_accept_replace_insufficient_collateral_fails() {
    run_test(|| {
        Replace::get_replace_request.mock_safe(|_| {
            let mut r = test_request();
            r.old_vault = ALICE;
            r.new_vault = Some(3);
            MockResult::Return(Ok(r))
        });
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|_id| {
            let mut vault = test_vault();
            vault.banned_until = None;
            MockResult::Return(Ok(vault))
        });
        ext::vault_registry::is_collateral_below_secure_threshold::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(true)));
        let collateral = 100_000;
        assert_noop!(
            accept_replace(ALICE, H256([0u8; 32]), collateral),
            Error::InsufficientCollateral
        );
    })
}

#[test]
fn test_auction_replace_bad_old_vault_id_fails() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|id| {
            MockResult::Return(if *id == ALICE {
                Err(Error::InvalidVaultID)
            } else {
                Ok(test_vault())
            })
        });
        ext::vault_registry::is_collateral_below_secure_threshold::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(false)));
        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(20_000_000));
        let collateral = 100_000;
        let btc_amount = 100;
        assert_noop!(
            auction_replace(ALICE, BOB, btc_amount, collateral),
            Error::InvalidVaultID
        );
    })
}

#[test]
fn test_auction_replace_bad_new_vault_id_fails() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|id| {
            MockResult::Return(if *id == ALICE {
                Ok(test_vault())
            } else {
                Err(Error::InvalidVaultID)
            })
        });
        ext::vault_registry::is_collateral_below_secure_threshold::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(false)));
        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(10_000_000));
        let collateral = 100_000;
        let btc_amount = 100;
        assert_noop!(
            auction_replace(ALICE, BOB, btc_amount, collateral),
            Error::InvalidVaultID
        );
    })
}

#[test]
fn test_auction_replace_insufficient_collateral_fails() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|id| {
            MockResult::Return(if *id == ALICE {
                Ok(test_vault())
            } else {
                Ok(test_vault())
            })
        });
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(false)));
        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(10_000_000));
        let collateral = 100_000;
        let btc_amount = 100;
        assert_noop!(
            auction_replace(ALICE, BOB, btc_amount, collateral),
            Error::InsufficientCollateral
        );
    })
}

//TODO(jaupe) uncomment this once the threshold calcs are centralised by dom
/*
#[test]
fn test_auction_replace_succeeds() {
    run_test(|| {
        ext::vault_registry::get_vault_from_id::<Test>.mock_safe(|id| {
            MockResult::Return(if *id == ALICE {
                Ok(test_vault())
            } else {
                Ok(test_vault())
            })
        });
        ext::vault_registry::auction_collateral_threshold::<Test>
            .mock_safe(|| MockResult::Return(1000));
        ext::vault_registry::secure_collateral_threshold::<Test>
            .mock_safe(|| MockResult::Return(1000));
        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(50_000_000));
        let collateral = 100_000_000_000;
        let btc_amount = 100_000_000_000;
        assert_eq!(auction_replace(ALICE, BOB, btc_amount, collateral), Ok(()));
        //TODO(jaupe) test persistent state
    })
}
*/

#[test]
fn test_execute_replace_bad_replace_id_fails() {
    run_test(|| {
        Replace::get_replace_request
            .mock_safe(|_| MockResult::Return(Err(Error::InvalidReplaceID)));

        let new_vault_id = ALICE;
        let replace_id = H256::zero();
        let tx_id = H256Le::zero();
        let tx_block_height = 1;
        let merkle_proof = Vec::new();
        let raw_tx = Vec::new();
        assert_eq!(
            execute_replace(
                new_vault_id,
                replace_id,
                tx_id,
                tx_block_height,
                merkle_proof,
                raw_tx
            ),
            Err(Error::InvalidReplaceID)
        );
    })
}

#[test]
fn test_execute_replace_replace_period_expired_fails() {
    run_test(|| {
        Replace::get_replace_request.mock_safe(|_| {
            let mut req = test_request();
            req.open_time = 100000;
            MockResult::Return(Ok(req))
        });

        let new_vault_id = ALICE;
        let replace_id = H256::zero();
        let tx_id = H256Le::zero();
        let tx_block_height = 1;
        let merkle_proof = Vec::new();
        let raw_tx = Vec::new();
        assert_eq!(
            execute_replace(
                new_vault_id,
                replace_id,
                tx_id,
                tx_block_height,
                merkle_proof,
                raw_tx
            ),
            Err(Error::ReplacePeriodExpired)
        );
    })
}

#[test]
fn test_cancel_replace_invalid_replace_id_fails() {
    run_test(|| {
        Replace::get_replace_request
            .mock_safe(|_| MockResult::Return(Err(Error::InvalidReplaceID)));

        let new_vault_id = ALICE;
        let replace_id = H256::zero();

        assert_eq!(
            cancel_replace(new_vault_id, replace_id),
            Err(Error::InvalidReplaceID)
        );
    })
}

//TODO(jaupe) tesst when current height is 0 and replace period is 1 as i think this can overflow
#[test]
fn test_cancel_replace_period_not_expired_fails() {
    run_test(|| {
        Replace::get_replace_request.mock_safe(|_| MockResult::Return(Ok(test_request())));
        Replace::current_height.mock_safe(|| MockResult::Return(1));
        Replace::replace_period.mock_safe(|| MockResult::Return(2));
        let new_vault_id = ALICE;
        let replace_id = H256::zero();

        assert_eq!(
            cancel_replace(new_vault_id, replace_id),
            Err(Error::ReplacePeriodNotExpired)
        );
    })
}

#[test]
fn test_request_replace_succeeds() {
    run_test(|| {
        let origin = ALICE;
        let vault = BOB;
        let amount = 6;
        let timeout = 10;
        let replace_id = H256::zero();
        let griefing_collateral = 10_000;

        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(test_vault())));

        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(100_000));
        ext::vault_registry::is_over_minimum_collateral::<Test>
            .mock_safe(|_| MockResult::Return(false));
        ext::collateral::lock_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));

        ext::vault_registry::increase_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::security::gen_secure_id::<Test>.mock_safe(|_| MockResult::Return(H256::zero()));
        assert_eq!(
            request_replace(origin, vault, amount, timeout, griefing_collateral),
            Ok(replace_id),
        );
    })
}

#[test]
fn test_withdraw_replace_succeeds() {
    run_test(|| {
        let vault_id = BOB;
        let replace_id = H256::zero();
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(test_vault())));

        Replace::get_replace_request.mock_safe(|_| {
            let mut replace = test_request();
            replace.old_vault = BOB;
            MockResult::Return(Ok(replace))
        });

        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(100_000));
        ext::vault_registry::is_over_minimum_collateral::<Test>
            .mock_safe(|_| MockResult::Return(false));
        ext::collateral::lock_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(false)));
        ext::vault_registry::increase_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::security::gen_secure_id::<Test>.mock_safe(|_| MockResult::Return(H256::zero()));
        ext::vault_registry::decrease_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        Replace::remove_replace_request.mock_safe(|_| MockResult::Return(()));
        assert_eq!(withdraw_replace(vault_id, replace_id), Ok(()));
    })
}

#[test]
fn test_accept_replace_succeeds() {
    run_test(|| {
        let vault_id = BOB;
        let replace_id = H256::zero();
        let collateral = 20_000;
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(test_vault())));

        Replace::get_replace_request.mock_safe(|_| {
            let mut replace = test_request();
            replace.old_vault = BOB;
            MockResult::Return(Ok(replace))
        });

        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(100_000));
        ext::vault_registry::is_over_minimum_collateral::<Test>
            .mock_safe(|_| MockResult::Return(false));
        ext::collateral::lock_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(false)));
        ext::vault_registry::increase_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::security::gen_secure_id::<Test>.mock_safe(|_| MockResult::Return(H256::zero()));
        ext::vault_registry::decrease_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        Replace::remove_replace_request.mock_safe(|_| MockResult::Return(()));
        ext::vault_registry::is_collateral_below_secure_threshold::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(false)));

        assert_eq!(accept_replace(vault_id, replace_id, collateral), Ok(()));
    })
}

#[test]
fn test_auction_replace_succeeds() {
    run_test(|| {
        let old_vault_id = BOB;
        let new_vault_id = ALICE;
        let btc_amount = 1000;
        let collateral = 20_000;
        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(test_vault())));

        Replace::get_replace_request.mock_safe(|_| {
            let mut replace = test_request();
            replace.old_vault = BOB;
            MockResult::Return(Ok(replace))
        });

        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(100_000));
        ext::vault_registry::is_over_minimum_collateral::<Test>
            .mock_safe(|_| MockResult::Return(false));
        ext::collateral::lock_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(true)));
        ext::vault_registry::increase_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::security::gen_secure_id::<Test>.mock_safe(|_| MockResult::Return(H256::zero()));
        ext::vault_registry::decrease_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        Replace::remove_replace_request.mock_safe(|_| MockResult::Return(()));
        ext::vault_registry::is_collateral_below_secure_threshold::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(false)));

        assert_eq!(
            auction_replace(old_vault_id, new_vault_id, btc_amount, collateral),
            Ok(())
        );
    })
}

#[test]
fn test_execute_replace_succeeds() {
    run_test(|| {
        let new_vault_id = ALICE;
        let replace_id = H256::zero();
        let tx_id = H256Le::zero();
        let tx_block_height = 1;
        let merkle_proof = Vec::new();
        let raw_tx = Vec::new();

        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(test_vault())));

        Replace::get_replace_request.mock_safe(|_| {
            let mut replace = test_request();
            replace.old_vault = BOB;
            replace.new_vault = Some(ALICE);
            MockResult::Return(Ok(replace))
        });

        ext::btc_relay::verify_transaction_inclusion::<Test>
            .mock_safe(|_, _, _| MockResult::Return(Ok(())));
        ext::btc_relay::validate_transaction::<Test>
            .mock_safe(|_, _, _, _| MockResult::Return(Ok(())));
        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(100_000));
        ext::vault_registry::is_over_minimum_collateral::<Test>
            .mock_safe(|_| MockResult::Return(false));
        ext::collateral::lock_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(true)));
        ext::vault_registry::increase_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::security::gen_secure_id::<Test>.mock_safe(|_| MockResult::Return(H256::zero()));
        ext::vault_registry::decrease_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        Replace::remove_replace_request.mock_safe(|_| MockResult::Return(()));
        ext::vault_registry::is_collateral_below_secure_threshold::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(false)));
        ext::vault_registry::replace_tokens::<Test>
            .mock_safe(|_, _, _, _| MockResult::Return(Ok(())));
        ext::collateral::release_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        assert_eq!(
            execute_replace(
                new_vault_id,
                replace_id,
                tx_id,
                tx_block_height,
                merkle_proof,
                raw_tx
            ),
            Ok(())
        );
    })
}

#[test]
fn test_cancel_replace_succeeds() {
    run_test(|| {
        let new_vault_id = ALICE;
        let replace_id = H256::zero();

        ext::vault_registry::get_vault_from_id::<Test>
            .mock_safe(|_| MockResult::Return(Ok(test_vault())));

        Replace::get_replace_request.mock_safe(|_| {
            let mut replace = test_request();
            replace.old_vault = BOB;
            replace.new_vault = Some(ALICE);
            replace.open_time = 2;
            MockResult::Return(Ok(replace))
        });
        Replace::current_height.mock_safe(|| MockResult::Return(10));
        Replace::replace_period.mock_safe(|| MockResult::Return(2));
        ext::btc_relay::verify_transaction_inclusion::<Test>
            .mock_safe(|_, _, _| MockResult::Return(Ok(())));
        ext::btc_relay::validate_transaction::<Test>
            .mock_safe(|_, _, _, _| MockResult::Return(Ok(())));
        ext::collateral::get_collateral_from_account::<Test>
            .mock_safe(|_| MockResult::Return(100_000));
        ext::vault_registry::is_over_minimum_collateral::<Test>
            .mock_safe(|_| MockResult::Return(false));
        ext::collateral::lock_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::vault_registry::is_vault_below_auction_threshold::<Test>
            .mock_safe(|_| MockResult::Return(Ok(true)));
        ext::vault_registry::increase_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        ext::security::gen_secure_id::<Test>.mock_safe(|_| MockResult::Return(H256::zero()));
        ext::vault_registry::decrease_to_be_redeemed_tokens::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(())));
        Replace::remove_replace_request.mock_safe(|_| MockResult::Return(()));
        ext::vault_registry::is_collateral_below_secure_threshold::<Test>
            .mock_safe(|_, _| MockResult::Return(Ok(false)));
        ext::vault_registry::replace_tokens::<Test>
            .mock_safe(|_, _, _, _| MockResult::Return(Ok(())));
        ext::collateral::release_collateral::<Test>.mock_safe(|_, _| MockResult::Return(Ok(())));
        assert_eq!(cancel_replace(new_vault_id, replace_id,), Ok(()));
    })
}
