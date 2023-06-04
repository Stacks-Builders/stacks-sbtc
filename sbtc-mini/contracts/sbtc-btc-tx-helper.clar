;; sbtc-btc-tx-helper will eventually be a custom clarity-bitcoin optimised for sBTC

(define-constant version-P2WPKH 0x04)
(define-constant version-P2WSH 0x05)
(define-constant version-P2TR 0x06)
(define-constant supported-address-versions (list version-P2WPKH version-P2WSH version-P2TR))

(define-public (was-segwit-tx-mined
	(burn-height uint) ;; bitcoin block height
	(tx (buff 4096)) ;; tx to check
	(header (buff 80)) ;; bitcoin block header
	(tx-index uint)
	(tree-depth uint)
	(wproof (list 14 (buff 32))) ;; merkle proof for wtxids
	(witness-merkle-root (buff 32))
	(witness-reserved-data (buff 32))
	(ctx (buff 1024)) ;; non-segwit coinbase tx, contains the witness root hash
	(cproof (list 14 (buff 32))) ;; merkle proof for coinbase tx
	;; proof and cproof trees could somehow be condensed into a single list
	;; because they converge at some point
	)
	(begin
		;; TODO: optimise to one call
		(try! (contract-call? .clarity-bitcoin was-segwit-tx-mined-compact
			burn-height
			tx
			header
			tx-index
			tree-depth
			wproof
			witness-merkle-root
			witness-reserved-data
			ctx
			cproof
		))
		(ok (merge
			(try! (contract-call? .clarity-bitcoin parse-wtx tx))
			{txid: (contract-call? .clarity-bitcoin get-txid tx)}
		))
	)
)

;; sbtc opcode and payload version
(define-constant unlock-script-prefix 0x3c00)
(define-constant unlock-script-base-length u60) ;; base script length when contract name length byte = 0

(define-private (unlock-script-check-length (contract-name-length-byte (optional (buff 1))) (script-length uint))
	(is-eq script-length (+ (buff-to-uint-be (unwrap! contract-name-length-byte false)) unlock-script-base-length))
)

;; Example witness:
;; 183c001a7321b74e2b6a7e949e6c4ad313035b16650950170075200046422d30ec92c568e21be4b9579cfed8e71ba0702122b014755ae0e23e3563ac
(define-private (find-unlock-witness-iter (witness (buff 128)) (unlock-script (optional (buff 128))))
	(begin
		(asserts! (is-none unlock-script) unlock-script)
		(asserts! (and
				(is-eq (slice? witness u1 u3) (some unlock-script-prefix))
				(is-eq (len witness) (+ (buff-to-uint-be (unwrap! (element-at? witness u24) none)) unlock-script-base-length))
			)
			none
		)
		(some witness)
	)
)

;; To be merged with the custom parse-wtx in the future
(define-read-only (find-protocol-unlock-witness (witnesses (list 8 (buff 128))))
	(fold find-unlock-witness-iter witnesses none)
)

(define-read-only (prepend-length (input (buff 32)))
	(concat (unwrap-panic (element-at (unwrap-panic (to-consensus-buff? (len input))) u16)) input)
)

(define-read-only (hashbytes-to-scriptpubkey (peg-wallet { version: (buff 1), hashbytes: (buff 32) }))
	(begin
		(asserts! (is-some (index-of? supported-address-versions (get version peg-wallet))) none)
		(some (concat (if (is-eq (get version peg-wallet) version-P2TR) 0x01 0x00) (prepend-length (get hashbytes peg-wallet))))
	)
)

;; TODO: Do we need the optional and hard coupling with PoX?
;; FIXME: Replace with latest PoX contract.
(define-read-only (get-peg-wallet-scriptpubkey (cycle (optional uint)))
	(hashbytes-to-scriptpubkey (unwrap! (contract-call? .sbtc-registry get-cycle-peg-wallet (default-to (contract-call? 'ST000000000000000000002AMW42H.pox-2 current-pox-reward-cycle) cycle)) none))
)
