(define-constant mock-pox-reward-wallet-1 { version: 0x01, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699 })

;; @name Querying volunteer can pre-register in cycle (n - 1) to register in cycle n
;; @mine-blocks-before 5
(define-public (test-pre-register)
	(begin
		(unwrap!
			(contract-call? .sbtc-stacking-pool signer-pre-register tx-sender u1000000000 mock-pox-reward-wallet-1)
			(err "Should have succeeded")
			)
		(ok true)
	)
)