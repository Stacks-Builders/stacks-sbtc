(define-constant mock-pox-reward-wallet-1 { version: 0x01, hashbytes: 0x0011223344556699001122334455669900112233445566990011223344556699 })

;; cycle windows
(define-constant disbursement 0x00)
(define-constant registration 0x01)
(define-constant voting 0x02)
(define-constant transfer 0x03)
(define-constant penalty 0x04)
(define-constant bad-peg-state 0x05)

;; @name Querying volunteer can pre-register in cycle (n - 1) to register in cycle n
;; @mine-blocks-before 70000
;; (define-public (test-pre-register)
;; 	(begin
;; 		(unwrap!
;; 			(contract-call? .sbtc-stacking-pool signer-pre-register u1000000000 mock-pox-reward-wallet-1)
;; 			(err "Should have succeeded")
;; 			)
;; 		(ok true)
;; 	)
;; )

;; @name Is protocol caller test (is not at first)
(define-public (test-is-protocol-caller)
	(if (is-ok (contract-call? .sbtc-stacking-pool is-protocol-caller))
		(err false)
		(ok true)
	)
)

;; @name Get current cycle stacker/signer pool, should return none
(define-public (test-get-current-cycle-pool-none)
    (begin
		(unwrap!
			(contract-call? .sbtc-stacking-pool get-current-cycle-pool)
			(ok true)
			)
		(err  "Should have succeeded")
	)
)

;; @name Get specific cycle stacker/signer pool, should return none
(define-public (test-get-cycle-pool-none)
	(begin
		(unwrap!
			(contract-call? .sbtc-stacking-pool get-specific-cycle-pool u0)
			(ok true)
			)
		(err  "Should have succeeded")
	)
)

;; @name Get current window
;; @mine-blocks-before 2100
(define-public (test-get-current-window)
	(if (is-eq registration (contract-call? .sbtc-stacking-pool get-current-window))
		(ok true)
		(err false)
	)
)

;; @name Get default signer in cycle
(define-public (test-get-signer-in-cycle)
	(if (is-eq u0 (get amount (contract-call? .sbtc-stacking-pool get-signer-in-cycle 'ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM u0)))
		(ok true)
		(err false)
	)
)