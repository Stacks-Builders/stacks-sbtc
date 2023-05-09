(define-constant err-peg-in-expired (err u500))
(define-constant err-not-a-peg-wallet (err u501))
(define-constant err-sequence-length-invalid (err u502))
(define-constant err-stacks-pubkey-invalid (err u503))

(define-constant wallet-1 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5)
(define-constant wallet-1-pubkey 0x03cd2cfdbd2ad9332828a7a13ef62cb999e063421c708e863a7ffed71fb61c88c9)

(define-constant mock-peg-wallet { version: 0x01, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699 })
(define-constant mock-peg-cycle u1)

;; [stacks pubkey, 33 bytes] OP_DROP [33 bytes] [33 bytes]
;; 03cd2cfdbd2ad9332828a7a13ef62cb999e063421c708e863a7ffed71fb61c88c9 (wallet-1 pubkey)
;; OP_DROP
;; 02fcba7ecf41bc7e1be4ee122d9d22e3333671eb0a3a87b5cdf099d59874e1940f
;; 02744b79efd72bec6e4cac8db6922a31f27674236dd8896403fb150aa112faf2b8
(define-constant mock-unlock-script-1 0x2103cd2cfdbd2ad9332828a7a13ef62cb999e063421c708e863a7ffed71fb61c88c9752102fcba7ecf41bc7e1be4ee122d9d22e3333671eb0a3a87b5cdf099d59874e1940f2102744b79efd72bec6e4cac8db6922a31f27674236dd8896403fb150aa112faf2b8)

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
(define-public (test-extract-data)
	;; TODO
	(let (
		(result (contract-call? .sbtc-peg-in-processor extract-data 0x mock-unlock-script-1))
		(reference (ok {
			recipient: wallet-1,
			value: u100,
			expiry-burn-height: (+ burn-block-height u10),
			peg-wallet: { version: 0x01, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699}
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
(define-public (test-peg-in)
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
		(unwrap! result (err {err: "Expect ok, got err", actual: result}))
		(ok true)
	)
)