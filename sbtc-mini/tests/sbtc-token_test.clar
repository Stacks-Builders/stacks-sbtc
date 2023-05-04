(define-constant wallet-1 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5)
(define-constant wallet-2 'ST2CY5V39NHDPWSXMW9QDT3HC3GD6Q6XX4CFRK9AG)
(define-constant test-mint-amount u10000000)
(define-constant test-total-supply (* u2 test-mint-amount))

(define-constant err-unauthorised (err u401))
(define-constant err-not-token-owner (err u4))

(define-private (assert-eq (result (response bool uint)) (compare (response bool uint)) (message (string-ascii 100)))
	(ok (asserts! (is-eq result compare) (err message)))
)

(define-private (assert-eq-string (result (response (string-ascii 32) uint)) (compare (response (string-ascii 32) uint)) (message (string-ascii 100)))
	(ok (asserts! (is-eq result compare) (err message)))
)

(define-private (assert-eq-uint (result (response uint uint)) (compare (response uint uint)) (message (string-ascii 100)))
	(ok (asserts! (is-eq result compare) (err message)))
)

(define-public (prepare-add-test-to-protocol)
	(contract-call? .sbtc-testnet-debug-controller set-protocol-contract (as-contract tx-sender) true)
)

(define-private (revoke-test-contract-protocol-status)
	(contract-call? .sbtc-testnet-debug-controller set-protocol-contract (as-contract tx-sender) false)
)

;; Prepare function called for all tests (unless overridden)
(define-public (prepare)
	(begin
		;; Add the test contract to the protocol contract set.
		(try! (prepare-add-test-to-protocol))
		;; Mint some tokens to test principals.
		(try! (contract-call? .sbtc-token protocol-mint test-mint-amount wallet-1))
		(try! (contract-call? .sbtc-token protocol-mint test-mint-amount wallet-2))
		(ok true)
	)
)

;; Prepare function that mints tokens and then revokes protocol contract
;; status of this test contract.
(define-public (prepare-and-revoke-access)
	(begin
		(try! (prepare))
		;; Remove the test contract from the protocol contract set.
		(revoke-test-contract-protocol-status)
	)
)

;; --- Protocol tests

;; @name Protocol can mint tokens
;; @prepare prepare-add-test-to-protocol
(define-public (test-protocol-mint)
	(contract-call? .sbtc-token protocol-mint u10000000 wallet-1)
)

;; @name Non-protocol contracts cannot mint tokens
;; @no-prepare
(define-public (test-protocol-mint-external)
	(assert-eq (contract-call? .sbtc-token protocol-mint u10000000 wallet-1) err-unauthorised "Should have failed")
)

;; @name Protocol can transfer user tokens
(define-public (test-protocol-transfer)
	(contract-call? .sbtc-token protocol-transfer u1 wallet-1 wallet-2)
)

;; @name Non-protocol cannot lock user tokens
;; @prepare prepare-and-revoke-access
(define-public (test-protocol-transfer-external)
	(assert-eq (contract-call? .sbtc-token protocol-transfer u1 wallet-1 wallet-2) err-unauthorised "Should have failed")
)

;; @name Protocol can lock user tokens
(define-public (test-protocol-lock)
	(contract-call? .sbtc-token protocol-lock u1 wallet-1)
)

;; @name Non-protocol cannot lock user tokens
;; @prepare prepare-and-revoke-access
(define-public (test-protocol-lock-external)
	(assert-eq (contract-call? .sbtc-token protocol-lock u1 wallet-1) err-unauthorised "Should have failed")
)

;; @name Can read locked user balance
(define-public (test-get-locked-balance)
	(begin
		(unwrap! (contract-call? .sbtc-token protocol-lock u1 wallet-1) (err "Could not lock tokens"))
		(try! (assert-eq-uint (contract-call? .sbtc-token get-balance-locked wallet-1) (ok u1) "Locked balance did not match"))
		(ok true)
	)
)

;; @name Protocol can unlock locked user tokens
(define-public (test-protocol-unlock)
	(begin
		(unwrap! (contract-call? .sbtc-token protocol-lock u1 wallet-1) (err "Token lock failed"))
		(unwrap! (contract-call? .sbtc-token protocol-unlock u1 wallet-1) (err "Token unlock failed"))
		(ok true)
	)
)

;; @name Non-protocol cannot unlock locked user tokens
(define-public (test-protocol-unlock-external)
	(begin
		(unwrap! (contract-call? .sbtc-token protocol-lock u1 wallet-1) (err "Token lock failed"))
		(unwrap! (revoke-test-contract-protocol-status) (err "Failed to revoke protocol status (not part of test)"))
		(assert-eq (contract-call? .sbtc-token protocol-unlock u1 wallet-1) err-unauthorised "Should have failed")
	)
)

;; @name Protocol can burn user tokens
(define-public (test-protocol-burn)
	(contract-call? .sbtc-token protocol-burn u1 wallet-1)
)

;; @name Non-protocol cannot burn user tokens
;; @prepare prepare-and-revoke-access
(define-public (test-protocol-burn-external)
	(assert-eq (contract-call? .sbtc-token protocol-burn u1 wallet-1) err-unauthorised "Should have failed")
)

;; @name Protocol can burn locked user tokens
(define-public (test-protocol-burn-locked)
	(begin
		(unwrap! (contract-call? .sbtc-token protocol-lock u1 wallet-1) (err "Token lock failed"))
		(unwrap! (contract-call? .sbtc-token protocol-burn-locked u1 wallet-1) (err "Burn locked failed"))
		(ok true)
	)
)

;; @name Non-protocol cannot burn locked user tokens
(define-public (test-protocol-burn-locked-external)
	(begin
		(unwrap! (contract-call? .sbtc-token protocol-lock u1 wallet-1) (err "Token lock failed"))
		(unwrap! (revoke-test-contract-protocol-status) (err "Failed to revoke protocol status (not part of test)"))
		(assert-eq (contract-call? .sbtc-token protocol-burn-locked u1 wallet-1) err-unauthorised "Should have failed")
	)
)

;; @name Protocol can set asset name
;; @prepare prepare-add-test-to-protocol
(define-public (test-protocol-set-name)
	(begin
		(unwrap! (contract-call? .sbtc-token protocol-set-name "_test_") (err "Could not set name"))
		(assert-eq-string (contract-call? .sbtc-token get-name) (ok "_test_") "Did not match new name")
	)
)

;; @name Non-protocol cannot set asset name
;; @no-prepare
(define-public (test-protocol-set-name-external)
	(begin
		(try! (assert-eq (contract-call? .sbtc-token protocol-set-name "_test_") err-unauthorised "Should have failed"))
		(assert-eq-string (contract-call? .sbtc-token get-name) (ok "sBTC Mini") "Name was not original name")
	)
)

;; @name Protocol can set asset symbol
;; @prepare prepare-add-test-to-protocol
(define-public (test-protocol-set-symbol)
	(begin
		(unwrap! (contract-call? .sbtc-token protocol-set-symbol "_test_") (err "Could not set symbol"))
		(assert-eq-string (contract-call? .sbtc-token get-symbol) (ok "_test_") "Did not match new symbol")
	)
)

;; @name Non-protocol cannot set asset symbol
;; @no-prepare
(define-public (test-protocol-set-symbol-external)
	(begin
		(try! (assert-eq (contract-call? .sbtc-token protocol-set-symbol "_test_") err-unauthorised "Should have failed"))
		(assert-eq-string (contract-call? .sbtc-token get-symbol) (ok "sBTC") "Symbol was not original symbol")
	)
)

;; @name Protocol can set asset token URI
;; @prepare prepare-add-test-to-protocol
(define-public (test-protocol-set-token-uri)
	(begin
		(unwrap! (contract-call? .sbtc-token protocol-set-token-uri (some u"_test_")) (err "Could not set token-uri"))
		(asserts! (is-eq (contract-call? .sbtc-token get-token-uri) (ok (some u"_test_"))) (err "Did not match new token-uri"))
		(ok true)
	)
)

;; @name Non-protocol cannot set asset token URI
;; @no-prepare
(define-public (test-protocol-set-token-uri-external)
	(begin
		(try! (assert-eq (contract-call? .sbtc-token protocol-set-token-uri (some u"_test_")) err-unauthorised "Should have failed"))
		(asserts! (is-eq (contract-call? .sbtc-token get-token-uri) (ok none)) (err "Token URI is not none"))
		(ok true)
	)
)

;; @name Protocol can mint tokens
;; @prepare prepare-add-test-to-protocol
(define-public (test-protocol-mint-many)
	(let (
		(result (unwrap! (contract-call? .sbtc-token protocol-mint-many
			(list {amount: u10000000, recipient: wallet-1} {amount: u10000000, recipient: wallet-2}))
			(err "Should not have failed")
			))
		)
		(asserts! (is-eq (len result) u2) (err "Result should have been length 2"))
		(asserts! (is-eq (element-at? result u0) (some (ok true))) (err "Mint 1 failed"))
		(asserts! (is-eq (element-at? result u1) (some (ok true))) (err "Mint 2 failed"))
		(ok true)
	)
)

;; @name Non-protocol contracts cannot mint tokens
;; @no-prepare
(define-public (test-protocol-mint-many-external)
	(ok
		(asserts! (is-eq 
			(contract-call? .sbtc-token protocol-mint-many (list {amount: u10000000, recipient: wallet-1} {amount: u10000000, recipient: wallet-2}))
			err-unauthorised
			)
		(err "Should have failed"))
	)
)

;; --- SIP010 tests

;; @name Token owner can transfer their tokens
;; @caller wallet_1
(define-public (test-transfer)
	(contract-call? .sbtc-token transfer u100 tx-sender wallet-2 none)
)

;; @name Cannot transfer someone else's tokens
;; @caller wallet_1
(define-public (test-transfer-external)
	(assert-eq (contract-call? .sbtc-token transfer u100 wallet-2 tx-sender none) (err u4) "Should have failed")
)

;; @name Can get name
(define-public (test-get-name)
	(assert-eq-string (contract-call? .sbtc-token get-name) (ok "sBTC Mini") "Name does not match")
)

;; @name Can get symbol
(define-public (test-get-symbol)
	(assert-eq-string (contract-call? .sbtc-token get-symbol) (ok "sBTC") "Name does not match")
)

;; @name Can get decimals
(define-public (test-get-decimals)
	(assert-eq-uint (contract-call? .sbtc-token get-decimals) (ok u8) "Decimals do not match")
)

;; @name Can user balance
(define-public (test-get-balance)
	(assert-eq-uint (contract-call? .sbtc-token get-balance wallet-1) (ok test-mint-amount) "Balance does not match")
)

;; @name User balance includes locked tokens
(define-public (test-get-balance-includes-locked-tokens)
	(begin
		(unwrap! (contract-call? .sbtc-token protocol-lock u1 wallet-1) (err "Could not lock tokens"))
		(assert-eq-uint (contract-call? .sbtc-token get-balance wallet-1) (ok test-mint-amount) "Balance does not match")
	)
)

;; @name Can get total supply
(define-public (test-get-total-supply)
	(assert-eq-uint (contract-call? .sbtc-token get-total-supply) (ok test-total-supply) "Total supply does not match")
)

;; @name Can get token URI
(define-public (test-get-token-uri)
	(ok (asserts! (is-eq (contract-call? .sbtc-token get-token-uri) (ok none)) (err "Total supply does not match")))
)
