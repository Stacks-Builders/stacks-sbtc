(define-constant err-peg-in-expired (err u500))
(define-constant err-not-a-peg-wallet (err u501))
(define-constant err-sequence-length-invalid (err u502))
(define-constant err-stacks-pubkey-invalid (err u503))

(define-constant wallet-1 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5)
(define-constant wallet-1-pubkey 0x03cd2cfdbd2ad9332828a7a13ef62cb999e063421c708e863a7ffed71fb61c88c9)

;; https://github.com/hirosystems/stacks.js/blob/c9e420e521cdc02d7ec81ea082f62d0a2d6c5e27/packages/stacking/src/constants.ts#L2

;; P2WPKH
;; WIF private key cVqZm6SNztZsZC75wAhmkewxxhCehq2QL7S8irdyWuBeyWAp21cj
;; hex private key f6588520e266c8ec43672fc97aa23a173831cd89be50823c7dca629b566d26b3
;; address bcrt1q5s4azffap92uc3qvujetg9ksgja424ef2hrsr5
;; address hash160 a42bd1253d0955cc440ce4b2b416d044bb555729

(define-constant mock-peg-wallet { version: 0x04, hashbytes: 0xa42bd1253d0955cc440ce4b2b416d044bb555729 })
(define-constant mock-peg-cycle u0)

;; [stacks pubkey, 33 bytes] OP_DROP [33 bytes] [33 bytes]
;; 03cd2cfdbd2ad9332828a7a13ef62cb999e063421c708e863a7ffed71fb61c88c9 (wallet-1 pubkey)
;; OP_DROP
;; 02fcba7ecf41bc7e1be4ee122d9d22e3333671eb0a3a87b5cdf099d59874e1940f
;; 02744b79efd72bec6e4cac8db6922a31f27674236dd8896403fb150aa112faf2b8
(define-constant mock-unlock-script-1 0x2103cd2cfdbd2ad9332828a7a13ef62cb999e063421c708e863a7ffed71fb61c88c9752102fcba7ecf41bc7e1be4ee122d9d22e3333671eb0a3a87b5cdf099d59874e1940f2102744b79efd72bec6e4cac8db6922a31f27674236dd8896403fb150aa112faf2b8)

;; createrawtransaction '[{"txid":"0000000000000000000000000000000000000000000000000000000000000000", "vout":0,"sequence":0}]' '[{"data":"03cd2cfdbd2ad9332828a7a13ef62cb999e063421c708e863a7ffed71fb61c88c9"}, {"bcrt1q5s4azffap92uc3qvujetg9ksgja424ef2hrsr5": 1.2}]'
(define-constant mock-op-return-tx-1 0x02000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000236a2103cd2cfdbd2ad9332828a7a13ef62cb999e063421c708e863a7ffed71fb61c88c9000e270700000000160014a42bd1253d0955cc440ce4b2b416d044bb55572900000000)
(define-constant mock-value-tx-1 u120000000)

(define-read-only (get-sbtc-balance (who principal))
	(unwrap! (contract-call? .sbtc-token get-balance who) u0)
)

(define-public (prepare-add-test-to-protocol)
	(contract-call? .sbtc-testnet-debug-controller set-protocol-contract (as-contract tx-sender) true)
)

(define-public (prepare)
	(begin
		;; Add the test contract to the protocol contract set.
		(try! (prepare-add-test-to-protocol))
		;; Add mock peg wallet adress to registry for test cycle
		(try! (contract-call? .sbtc-registry insert-cycle-peg-wallet mock-peg-cycle mock-peg-wallet))
		(ok true)
	)
)

;; @assert-event print {data: {expiry-burn-height: u17, peg-wallet: {hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699, version: 0x01}, recipient: ST000000000000000000002AMW42H, value: u100}, event: "peg-in", wtxid: 0x0011223344556677889900112233445566778899001122334455667788990011}


(define-public (test-extract-principal)
	(ok (asserts!
		(is-eq (contract-call? .sbtc-peg-in-processor extract-principal mock-unlock-script-1 u1) (ok wallet-1))
		(err "Extraction failed")
	))
)

(define-public (test-extract-principal-invalid-length)
	(ok (asserts!
		(is-eq
			(contract-call? .sbtc-peg-in-processor extract-principal 0x03cd2cfdbd2ad9332828a7a13ef62cb999e063421c708e863a7ffed71fb61c88c9 u1)
			err-sequence-length-invalid
		)
		(err "Should have failed with err-sequence-length-invalid")
	))
)

(define-public (test-extract-principal-invalid-pubkey)
	(ok (asserts!
		(is-eq
			(contract-call? .sbtc-peg-in-processor extract-principal 0x2100cd2cfdbd2ad9332828a7a13ef62cb999e063421c708e863a7ffed71fb61c88c9 u1)
			err-stacks-pubkey-invalid
		)
		(err "Should have failed with err-stacks-pubkey-invalid")
	))
)

;; @name Can extract data from a transaction and unlock script
(define-public (disabled-test-extract-data)
	;; TODO
	(let (
		(result (contract-call? .sbtc-peg-in-processor extract-data 0x mock-unlock-script-1))
		(reference (ok {
			recipient: wallet-1,
			value: mock-value-tx-1,
			expiry-burn-height: (+ burn-block-height u10),
			;;peg-wallet: { version: 0x01, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699}
		}))
		)
		(ok (asserts!
			(is-eq result reference)
			(err {err: "Expected to be equal", expected: reference, actual: result}))
		)
	)
)

;; @mine-blocks-before 5
;; @print events
(define-public (disabled-test-peg-in-reveal)
	(let ((result (contract-call? .sbtc-peg-in-processor complete-peg-in
			mock-peg-cycle ;; burn-height
			0x11 ;; tx
			mock-unlock-script-1 ;; p2tr-unlock-script
			0x22 ;; header
			u1 ;; tx-index
			u1 ;; tree-depth
			(list 0x33 0x44) ;; wproof
			0x55 ;; ctx
			(list 0x55 0x66) ;; cproof
			)))
		(unwrap! result (err {err: "Expect ok, got err", actual: (some result)}))
		(asserts! (is-eq (get-sbtc-balance wallet-1) mock-value-tx-1) (err {err: "User did not receive the expected sBTC", actual: none}))
		(ok true)
	)
)

;; @mine-blocks-before 5
;; @print events
(define-public (test-peg-in-op-return)
	(let ((result (contract-call? .sbtc-peg-in-processor complete-peg-in
			mock-peg-cycle ;; burn-height
			mock-op-return-tx-1 ;; tx
			0x ;; p2tr-unlock-script
			0x22 ;; header
			u1 ;; tx-index
			u1 ;; tree-depth
			(list 0x33 0x44) ;; wproof
			0x55 ;; ctx
			(list 0x55 0x66) ;; cproof
			)))
		(unwrap! result (err {err: "Expect ok, got err", actual: (some result)}))
		(asserts! (is-eq (get-sbtc-balance wallet-1) mock-value-tx-1) (err {err: "User did not receive the expected sBTC", actual: none}))
		(ok true)
	)
)