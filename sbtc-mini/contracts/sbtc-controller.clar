(define-constant contract-deployer tx-sender)
(define-constant err-unauthorised (err u401))

(define-map privileged-protocol-principals principal bool)
;; FIXME: Potential issue in that the contract-deployer can mess with the registry state
;;        before bootstrapping the protocol.
(map-set privileged-protocol-principals contract-deployer true)

(define-read-only (is-protocol-caller (who principal))
	(ok (asserts! (default-to false (map-get? privileged-protocol-principals who)) err-unauthorised))
)

(define-private (set-protocol-iter (entry {contract: principal, enabled: bool}))
	(and
		;; Only contract principals can be part of the protocol
		(is-some (get name (unwrap! (principal-destruct? (get contract entry)) false)))
		(map-set privileged-protocol-principals (get contract entry) (get enabled entry))
	)
)

(define-public (upgrade (protocol-principals (list 20 {contract: principal, enabled: bool})))
	(begin
		(try! (is-protocol-caller contract-caller))
		(map-delete privileged-protocol-principals contract-deployer)
		(ok (map set-protocol-iter protocol-principals))
	)
)

