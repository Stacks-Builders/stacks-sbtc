(define-constant wallet-1 'ST1SJ3DTE5DN7X54YDH5D64R3BCB6A2AG2ZQ8YPD5)
(define-constant wallet-1-pubkey 0x03cd2cfdbd2ad9332828a7a13ef62cb999e063421c708e863a7ffed71fb61c88c9)

(define-constant mock-peg-wallet { version: 0x01, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699 })
(define-constant mock-peg-cycle u1)

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

;; @mine-blocks-before 5
(define-public (test-peg-in)
	(begin
		(try! (contract-call? .sbtc-peg-in-processor complete-peg-in
			mock-peg-cycle ;; burn-height
			0x11 ;; tx
			(concat 0xff wallet-1-pubkey) ;; p2tr-unlock-script
			0x22 ;; header
			u1 ;; tx-index
			u1 ;; tree-depth
			(list 0x33 0x44) ;; wproof
			0x55 ;; ctx
			(list 0x55 0x66) ;; cproof
			));; (err "Peg-in failed"))
		(ok true)
	)
)