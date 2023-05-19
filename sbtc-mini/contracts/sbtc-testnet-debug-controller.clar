;; THIS CONTRACT SHOULD ONLY BE USED FOR DEVELOPMENT PURPOSES
;;
;; Debug controller contract that can be made part
;; of the protocol during deploy.
;; This contract can trigger protocol upgrades
;; by the contract deployer or any principals it
;; defines later.

;; Add some safety to prevent accidental deployment on mainnet
(asserts! (is-eq chain-id u2147483648) (err "This contract can only be deployed on testnet"))

(define-constant err-not-debug-controller (err u900))
(define-constant err-no-transactions (err u901))

(define-constant OP_RETURN 0x6a)

(define-map debug-controllers principal bool)
(map-set debug-controllers tx-sender true)

(define-read-only (is-debug-controller (controller principal))
	(ok (asserts! (default-to false (map-get? debug-controllers controller)) err-not-debug-controller))
)

;; #[allow(unchecked_data)]
(define-public (set-debug-controller (who principal) (enabled bool))
	(begin
		(try! (is-debug-controller tx-sender))
		(ok (map-set debug-controllers who enabled))
	)
)

;; #[allow(unchecked_data)]
(define-public (set-protocol-contract (contract principal) (enabled bool))
	(begin
		(try! (is-debug-controller tx-sender))
		(contract-call? .sbtc-controller upgrade (list {contract: contract, enabled: enabled}))
	)
)

(define-read-only (reverse-buff32-iter (a (buff 1)) (b (buff 32)))
	(unwrap-panic (as-max-len? (concat a b) u32))
)

;; This one consumes 60,681 runtime units
(define-read-only (reverse-buff32-v2 (input (buff 32)))
	(fold reverse-buff32-iter input 0x)
)

(define-private (reverse-buff16 (input (buff 16)))
	(unwrap-panic (slice? (unwrap-panic (to-consensus-buff? (buff-to-uint-le input))) u1 u17))
)

;; This function consumes 12,675 runtime units.
;; It is 4.8x more efficient than a fold.
(define-read-only (reverse-buff32 (input (buff 32)))
	(unwrap-panic (as-max-len? (concat
		(reverse-buff16 (unwrap-panic (as-max-len? (unwrap-panic (slice? input u16 u32)) u16)))
		(reverse-buff16 (unwrap-panic (as-max-len? (unwrap-panic (slice? input u0 u16)) u16)))
	) u32))
)

(define-read-only (calculate-internal-txid? (transaction (optional (buff 4096))))
	(match transaction unwrapped (some (sha256 (sha256 unwrapped))) none)
)

(define-read-only (calculate-merkle-branch?	 (left (optional (buff 32))) (right (optional (buff 32))))
	(match left inner-left
		(match right inner-right (some (sha256 (sha256 (concat inner-left inner-right)))) left)
		right
	)
)

(define-constant solo-block-version-le 0x02000000)
(define-constant solo-block-prevhash-le 0x0000000000000000000000000000000000000000000000000000000000000000)
(define-constant solo-block-timestamp-le 0x00000000)
(define-constant solo-block-bits-le 0x00000000)
(define-constant solo-block-nonce-le 0x00000000)

 ;; version || PrevHash || MerkleRoot || Timestamp || Bits || Nonce

(define-read-only (create-solo-burnchain-header-buff (merkle-root-le (buff 32)))
	(concat
		solo-block-version-le
	(concat
		solo-block-prevhash-le
	(concat
		merkle-root-le
	(concat
		solo-block-timestamp-le
	(concat
		solo-block-bits-le
		solo-block-nonce-le
	)))))
)

(define-read-only (calculate-solo-burnchain-header-hash (merkle-root-le (buff 32)))
	(sha256 (sha256 (create-solo-burnchain-header-buff merkle-root-le)))
)

(define-public (simulate-mine-solo-burnchain-block (burn-height uint) (transactions (list 4 (buff 4096))))
	(contract-call? .clarity-bitcoin mock-add-burnchain-block-header-hash burn-height
		(reverse-buff32 (calculate-solo-burnchain-header-hash
			(unwrap! (calculate-merkle-branch?
				(calculate-merkle-branch?
					(calculate-internal-txid? (element-at? transactions u0))
					(calculate-internal-txid? (element-at? transactions u1))
				)
				(calculate-merkle-branch?
					(calculate-internal-txid? (element-at? transactions u2))
					(calculate-internal-txid? (element-at? transactions u3))
				)
			) err-no-transactions)
		)
	))
)

;; TODO
(define-read-only (generate-segwit-coinbase-transaction)
	0x00
)

(define-read-only (generate-segwit-commit-structure (segwit-root-hash (buff 32)) (witness-reserved-data (buff 32)))
	(concat
		0x6a24aa21a9ed
		(sha256 (sha256 (concat segwit-root-hash witness-reserved-data)))
	)
)

;; TODO
(define-public (simulate-mine-solo-burnchain-block-segwit (burn-height uint) (segwit-transactions (list 3 (buff 4096))))
	(contract-call? .clarity-bitcoin mock-add-burnchain-block-header-hash burn-height
		(reverse-buff32 (calculate-solo-burnchain-header-hash
			(unwrap! (calculate-merkle-branch?
				(calculate-merkle-branch?
					(calculate-internal-txid? (some (generate-segwit-coinbase-transaction)))
					(calculate-internal-txid? (element-at? segwit-transactions u0))
				)
				(calculate-merkle-branch?
					(calculate-internal-txid? (element-at? segwit-transactions u1))
					(calculate-internal-txid? (element-at? segwit-transactions u2))
				)
			) err-no-transactions)
		)
	))
)

;; Some test vectors:

;; coinbase tx: 0x02000000010000000000000000000000000000000000000000000000000000000000000000ffffffff23036f18250418b848644d65726d61696465722046545721010000686d20000000000000ffffffff02edfe250000000000160014c035e789d9efffa10aa92e93f48f29b8cfb224c20000000000000000266a24aa21a9ed260ac9521c6b0c1b09e438319b5cb3377911764f156e44da61b1ab820f75104c00000000
;; coinbase txid: 0x5040e545e752f92999e659a9a6c03081f7ff6fe38ed071650d6463668d11bbf1
;; tx: 0x0200000000010218f905443202116524547142bd55b69335dfc4e4c66ff3afaaaab6267b557c4b030000000000000000e0dbdf1039321ab7a2626ca5458e766c6107690b1a1923e075c4f691cc4928ac0000000000000000000220a10700000000002200208730dbfaa29c49f00312812aa12a62335113909711deb8da5ecedd14688188363c5f26010000000022512036f4ff452cb82e505436e73d0a8b630041b71e037e5997290ba1fe0ae7f4d8d50140a50417be5a056f63e052294cb20643f83038d5cd90e2f90c1ad3f80180026cb99d78cd4480fadbbc5b9cad5fb2248828fb21549e7cb3f7dbd7aefd2d541bd34f0140acde555b7689eae41d5ccf872bb32a270893bdaa1defc828b76c282f6c87fc387d7d4343c5f7288cfd9aa5da0765c7740ca97e44a0205a1abafa279b530d5fe36d182500
;; txid: 0x04117dc370c45b8a44bf86a3ae4fa8d0b186b5b27d50939cda7501723fa12ec6
;; merkle root: 0x400cacdb9a74f8a7fd6a22ed21ea999acc914b83f1f51eecf2426cf7db5eb7fd
;; header: 0x020000000000000000000000000000000000000000000000000000000000000000000000400cacdb9a74f8a7fd6a22ed21ea999acc914b83f1f51eecf2426cf7db5eb7fd000000000000000000000000
;; header hash: 0x66aedd8941ccaa1dc49846a43e275d7a186df2c206e3b4d7f335d8ff09dcbfaf

;; Mine block at height 3:
;; (contract-call? .sbtc-testnet-debug-controller simulate-mine-solo-burnchain-block u3 (list 0x02000000010000000000000000000000000000000000000000000000000000000000000000ffffffff23036f18250418b848644d65726d61696465722046545721010000686d20000000000000ffffffff02edfe250000000000160014c035e789d9efffa10aa92e93f48f29b8cfb224c20000000000000000266a24aa21a9ed260ac9521c6b0c1b09e438319b5cb3377911764f156e44da61b1ab820f75104c00000000 0x0200000000010218f905443202116524547142bd55b69335dfc4e4c66ff3afaaaab6267b557c4b030000000000000000e0dbdf1039321ab7a2626ca5458e766c6107690b1a1923e075c4f691cc4928ac0000000000000000000220a10700000000002200208730dbfaa29c49f00312812aa12a62335113909711deb8da5ecedd14688188363c5f26010000000022512036f4ff452cb82e505436e73d0a8b630041b71e037e5997290ba1fe0ae7f4d8d50140a50417be5a056f63e052294cb20643f83038d5cd90e2f90c1ad3f80180026cb99d78cd4480fadbbc5b9cad5fb2248828fb21549e7cb3f7dbd7aefd2d541bd34f0140acde555b7689eae41d5ccf872bb32a270893bdaa1defc828b76c282f6c87fc387d7d4343c5f7288cfd9aa5da0765c7740ca97e44a0205a1abafa279b530d5fe36d182500))

;; Verify transaction was mined:
;; (contract-call? .clarity-bitcoin was-tx-mined-compact u3 0x0200000000010218f905443202116524547142bd55b69335dfc4e4c66ff3afaaaab6267b557c4b030000000000000000e0dbdf1039321ab7a2626ca5458e766c6107690b1a1923e075c4f691cc4928ac0000000000000000000220a10700000000002200208730dbfaa29c49f00312812aa12a62335113909711deb8da5ecedd14688188363c5f26010000000022512036f4ff452cb82e505436e73d0a8b630041b71e037e5997290ba1fe0ae7f4d8d50140a50417be5a056f63e052294cb20643f83038d5cd90e2f90c1ad3f80180026cb99d78cd4480fadbbc5b9cad5fb2248828fb21549e7cb3f7dbd7aefd2d541bd34f0140acde555b7689eae41d5ccf872bb32a270893bdaa1defc828b76c282f6c87fc387d7d4343c5f7288cfd9aa5da0765c7740ca97e44a0205a1abafa279b530d5fe36d182500 0x020000000000000000000000000000000000000000000000000000000000000000000000400cacdb9a74f8a7fd6a22ed21ea999acc914b83f1f51eecf2426cf7db5eb7fd000000000000000000000000 {tx-index: u1, hashes: (list 0x5040e545e752f92999e659a9a6c03081f7ff6fe38ed071650d6463668d11bbf1), tree-depth: u2})

;; Verify coinbase:
;; (contract-call? .clarity-bitcoin was-tx-mined-compact u3 0x02000000010000000000000000000000000000000000000000000000000000000000000000ffffffff23036f18250418b848644d65726d61696465722046545721010000686d20000000000000ffffffff02edfe250000000000160014c035e789d9efffa10aa92e93f48f29b8cfb224c20000000000000000266a24aa21a9ed260ac9521c6b0c1b09e438319b5cb3377911764f156e44da61b1ab820f75104c00000000 0x020000000000000000000000000000000000000000000000000000000000000000000000400cacdb9a74f8a7fd6a22ed21ea999acc914b83f1f51eecf2426cf7db5eb7fd000000000000000000000000 {tx-index: u0, hashes: (list 0x04117dc370c45b8a44bf86a3ae4fa8d0b186b5b27d50939cda7501723fa12ec6), tree-depth: u1})
