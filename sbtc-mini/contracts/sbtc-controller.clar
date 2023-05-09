;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;; Cons, Vars & Maps ;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

;;;;;;;;;;;;;;;;;
;;; constants ;;;
;;;;;;;;;;;;;;;;;

(define-constant contract-deployer tx-sender)
(define-constant err-unauthorised (err u401))

;;;;;;;;;;;;;;;;;
;;; variables ;;;
;;;;;;;;;;;;;;;;;



;;;;;;;;;;;;
;;; maps ;;;
;;;;;;;;;;;;

(define-map privileged-protocol-principals principal bool)
(map-set privileged-protocol-principals tx-sender true)



;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;; Read-Only Funcs ;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;

(define-read-only (is-protocol-caller (who principal))
	(ok (asserts! (default-to false (map-get? privileged-protocol-principals who)) err-unauthorised))
)



;;;;;;;;;;;;;;;;;;;;;;;;
;;;;; Public Funcs ;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;

(define-public (upgrade (protocol-principals (list 20 {contract: principal, enabled: bool})))
	(begin
		(try! (is-protocol-caller contract-caller))
		(map-delete privileged-protocol-principals contract-deployer)
		(ok (map set-protocol-iter protocol-principals))
	)
)

;; to-do: add a function to update the peg state (only current pool contract can call this?)
;; Update Peg State
;; @desc: Updates the peg state to the value passed in



;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;; Private Funcs ;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;

(define-private (set-protocol-iter (entry {contract: principal, enabled: bool}))
	(and
		;; Only contract principals can be part of the protocol
		(is-some (get name (unwrap! (principal-destruct? (get contract entry)) false)))
		(map-set privileged-protocol-principals (get contract entry) (get enabled entry))
	)
)
