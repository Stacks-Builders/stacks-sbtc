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
;; hex public key 030046422d30ec92c568e21be4b9579cfed8e71ba0702122b014755ae0e23e3563
;; address bcrt1q5s4azffap92uc3qvujetg9ksgja424ef2hrsr5
;; address hash160 a42bd1253d0955cc440ce4b2b416d044bb555729

(define-constant mock-peg-wallet { version: 0x04, hashbytes: 0xbfbe43457367d8acd108dcf1a8ca195ba6ba4ba9 })
(define-constant mock-peg-cycle u0)
(define-constant mock-burnchain-height u3)

;; op byte is "<" (0x3c)
;; version is always 0
;; [op 1 byte] [version 1 byte] [address version 1 byte] [address 20 bytes] [length prefixed contract name] OP_DROP [33 bytes] OP_CHECKSIG

;; 3c (sbtc opcode)
;; 00 (payload version)
;; 1a (wallet-1 address version)
;; 7321b74e2b6a7e949e6c4ad313035b1665095017 (wallet-1 hashbytes)
;; 00 (contract name length)
;; OP_DROP
;; 0046422d30ec92c568e21be4b9579cfed8e71ba0702122b014755ae0e23e3563
;; OP_CHECKSIG
(define-constant mock-unlock-script-1 0x183c001a7321b74e2b6a7e949e6c4ad313035b16650950170075200046422d30ec92c568e21be4b9579cfed8e71ba0702122b014755ae0e23e3563ac)
(define-constant mock-tx-1 0x02000000000101bc3ef1d3826d9432f400840bbfc91931e47cf4aa592821326294c1f1d8cb245b0100000000fdffffff010065cd1d00000000160014bfbe43457367d8acd108dcf1a8ca195ba6ba4ba90340000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f3c183c001a7321b74e2b6a7e949e6c4ad313035b16650950170075200046422d30ec92c568e21be4b9579cfed8e71ba0702122b014755ae0e23e3563ac41c01dae61a4a8f841952be3a511502d4f56e889ffa0685aa0098773ea2d4309f62474708f439116be919de13c6d3200d2305fcbdf5a9e7d2c079e85b427bb110e9000000000)
(define-constant mock-wtxid-1 0x94a60ceec0be7c17b0b6d924b8c8aea8be49c003d32b831e256d035356b253b8)
(define-constant mock-txid-1 0xf07c86721f795087e2975df2b42ea04e4f34248108fbb225872f8ec9d1914cc7)
(define-constant mock-value-tx-1 u500000000)

(define-constant mock-witness-root-hash-1 0x94a60ceec0be7c17b0b6d924b8c8aea8be49c003d32b831e256d035356b253b8)
(define-constant mock-coinbase-witness-reserved-data 0x)

(define-constant mock-coinbase-tx-1 0x020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff03016500ffffffff0200f2052a010000002251205444612a122cd09b4b3457d46c149a23d8685fb7d3aac61ea7eee8449555293b0000000000000000266a24aa21a9edab124e70e4a18e72f2ac6f635aebb08862d5b5b1c0519cd9f3222a24a48482560120000000000000000000000000000000000000000000000000000000000000000000000000)
(define-constant mock-coinbase-txid-1 0x7888bc71f3497f187f708912c5cf24cab1d99a83b04d0a63c14150cc51d62705)

(define-constant mock-txid-merkle-root-hash-1-le 0xc3a195310679b98fbe7e3d4f2603878746af28acf32657bb69e50a60f1670f50)

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
		;; Mine a fake burnchain block that includes mock transactions
		;;(try! (contract-call? .sbtc-testnet-debug-controller simulate-mine-solo-burnchain-block mock-burnchain-height (list mock-tx-1)))
		(unwrap! (contract-call? .clarity-bitcoin mock-add-burnchain-block-header-hash mock-burnchain-height 0x8edbd8533f28188fc25d3bec31e72206fff777334bdaa5cf941ea245326381d3) (err u1122))
		(ok true)
	)
)

;; @assert-event print {data: {expiry-burn-height: u17, peg-wallet: {hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699, version: 0x01}, recipient: ST000000000000000000002AMW42H, value: u100}, event: "peg-in", wtxid: 0x0011223344556677889900112233445566778899001122334455667788990011}


(define-public (test-extract-principal)
	(ok (asserts!
		(is-eq (contract-call? .sbtc-peg-in-processor extract-principal mock-unlock-script-1 u3) (some wallet-1))
		(err "Extraction failed")
	))
)

(define-public (test-extract-principal-invalid-length)
	(ok (asserts!
		(is-eq
			(contract-call? .sbtc-peg-in-processor extract-principal 0x03cd2cfdbd2ad9332828a7a13ef62cb999e063421c708e863a7ffed71fb61c88c9 u1)
			none
		)
		(err "Should have failed with err-sequence-length-invalid")
	))
)

(define-public (test-extract-principal-invalid-pubkey)
	(ok (asserts!
		(is-eq
			(contract-call? .sbtc-peg-in-processor extract-principal 0x2100cd2cfdbd2ad9332828a7a13ef62cb999e063421c708e863a7ffed71fb61c88c9 u1)
			none
		)
		(err "Should have failed with err-stacks-pubkey-invalid")
	))
)

;; @name Can extract data from a transaction and unlock script
;; (define-public (disabled-test-extract-data)
;; 	;; TODO
;; 	(let (
;; 		(result (contract-call? .sbtc-peg-in-processor extract-data 0x mock-unlock-script-1))
;; 		(reference (ok {
;; 			recipient: wallet-1,
;; 			value: mock-value-tx-1,
;; 			expiry-burn-height: (+ burn-block-height u10),
;; 			;;peg-wallet: { version: 0x01, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699}
;; 		}))
;; 		)
;; 		(ok (asserts!
;; 			(is-eq result reference)
;; 			(err {err: "Expected to be equal", expected: reference, actual: result}))
;; 		)
;; 	)
;; )

;; @mine-blocks-before 5
;; @print events
(define-public (disabled-test-peg-in-reveal)
	(let ((result (contract-call? .sbtc-peg-in-processor complete-peg-in
			mock-peg-cycle ;; burn-height
			0x11 ;; tx
			0x22 ;; header
			u1 ;; tx-index
			u1 ;; tree-depth
			(list 0x33 0x44) ;; wproof
			0x
			0x
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
(define-public (test-peg-in-reveal)
	(let ((result (contract-call? .sbtc-peg-in-processor complete-peg-in
			mock-burnchain-height ;; burn-height
			mock-tx-1 ;; tx
			;; header hash: 0x8edbd8533f28188fc25d3bec31e72206fff777334bdaa5cf941ea245326381d3
			0x020000000000000000000000000000000000000000000000000000000000000000000000c3a195310679b98fbe7e3d4f2603878746af28acf32657bb69e50a60f1670f50000000000000000000000000 ;; header
			u0 ;; tx-index
			u0 ;; tree-depth
			(list 0x7888bc71f3497f187f708912c5cf24cab1d99a83b04d0a63c14150cc51d62705) ;; wproof
			mock-witness-root-hash-1
			mock-coinbase-witness-reserved-data
			mock-coinbase-tx-1 ;; ctx
			(list mock-txid-1) ;; cproof
			)))
		(unwrap! result (err {err: "Expect ok, got err", actual: (some result)}))
		(asserts! (is-eq (get-sbtc-balance wallet-1) mock-value-tx-1) (err {err: "User did not receive the expected sBTC", actual: none}))
		(ok true)
	)
)

(define-public (test-temp-find-protocol-unlock-witness)
  (ok (asserts! (is-eq
      (some 0x183c001a7321b74e2b6a7e949e6c4ad313035b16650950170075200046422d30ec92c568e21be4b9579cfed8e71ba0702122b014755ae0e23e3563ac)
      (contract-call? .sbtc-btc-tx-helper find-protocol-unlock-witness (list 0x000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f000102030405060708090a0b0c0d0e0f 0x183c001a7321b74e2b6a7e949e6c4ad313035b16650950170075200046422d30ec92c568e21be4b9579cfed8e71ba0702122b014755ae0e23e3563ac 0xc01dae61a4a8f841952be3a511502d4f56e889ffa0685aa0098773ea2d4309f62474708f439116be919de13c6d3200d2305fcbdf5a9e7d2c079e85b427bb110e90))
      )
    (err "Could not find the witness (should have returned the second item)")
  ))
)
