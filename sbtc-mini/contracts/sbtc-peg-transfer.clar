;; sbtc-mini-peg-transfer
;; peg-transfer processor for handing-off pegged-BTC from threshold-wallet (n) to newly-voted-for threshold-wallet (n+1)

;; Handoff Commit/Fund -> On BTC
;; 1. Stackers/signers in cycle/pool N create & fund a Taproot address/script with the current peg-balance that allows for two things:
;;   2. The transaction can be consumed (transferred from wallet n to n+1) a single signature from any of the stackers/signers in cycle/pool N+1
;;   3. The transaction is NOT picked up by the end of the transfer window in n & is reclaimed by the stackers/signers in cycle/pool N

;; Handoff Reveal -> On STX
;; 2. The transaction is consumed & the pegged-BTC is succesfully transferred to the new threshold-wallet (n+1)
;;   2.a. Any observer can verify transfer with a call to .sbtc contracts with the Bitcoin txid of the transfer transaction
;;   This will mark a succesful transfer window & the current pool is moved to the audit/penalty window

;; Handoff Reclaim/Penalty -> On BTC/STX
;; 3. The transaction is NOT consumed & the pegged-BTC is NOT transferred to the new threshold-wallet (n+1)

;; We don't know what that transaction type will look like: 1 input from previous address that goes to the next address
;; What is the peg-wallet address look like? 
;; How are / will the signers going to consolidate inputs/outputs?
;; Or is this done over many validations?
;; How do we keep track of the pegged-BTC balance?

;; cycle windows
(define-constant disbursement 0x00)
(define-constant registration 0x01)
(define-constant voting 0x02)
(define-constant transfer 0x03)
(define-constant penalty 0x04)
(define-constant bad-peg-state 0x05)

(define-constant err-current-pool-not-found (err u0))
(define-constant err-current-threshold-wallet (err u1))
(define-constant err-previous-pool-not-found (err u2))
(define-constant err-pool-cycle (err u3))
(define-constant err-previous-threshold-wallet (err u4))
(define-constant err-parsing-btc-tx (err u5))
(define-constant err-tx-not-mined (err u6))
(define-constant err-not-in-transfer-window (err u7))
(define-constant err-balance-already-transferred (err u8))
(define-constant err-wrong-pubkey (err u9))
(define-constant err-peg-balance-not-sufficient (err u10))
(define-constant err-threshold-to-scriptpubkey (err u11))

;; Placeholder to make sbtc-stacking-pool happy
(define-public (relay-handoff-fulfillment
    (burn-height uint)
	(tx (buff 1024))
	(header (buff 80))
	(tx-index uint)
	(tree-depth uint)
	(wproof (list 14 (buff 32)))
    (witness-merkle-root (buff 32))
    (witness-reserved-data (buff 32))
	(ctx (buff 1024))
	(cproof (list 14 (buff 32))))
    (let 
        (
            (current-cycle (contract-call? 'SP000000000000000000002Q6VF78.pox-2 current-pox-reward-cycle))
            (current-pool-unwrapped (unwrap! (contract-call? .sbtc-stacking-pool get-current-cycle-pool) err-current-pool-not-found))
            (current-threshold-wallet (unwrap! (get threshold-wallet current-pool-unwrapped) err-current-threshold-wallet))
            (current-threshold-version (get version current-threshold-wallet))
            (current-threshold-hashbytes (get hashbytes current-threshold-wallet))

            (previous-cycle (- current-cycle u1))
            (previous-pool-unwrapped (unwrap! (contract-call? .sbtc-stacking-pool get-specific-cycle-pool previous-cycle) err-previous-pool-not-found))
            (previous-threshold-wallet (get hashbytes (unwrap! (get threshold-wallet previous-pool-unwrapped) err-previous-threshold-wallet)))
            (previous-pool-balance-transferred (get balance-transferred previous-pool-unwrapped))

            (cycle-peg-balance (contract-call? .sbtc-registry get-peg-balance))
            (parsed-tx (unwrap! (contract-call? .clarity-bitcoin parse-tx tx) err-parsing-btc-tx))
            (tx-outputs (get outs parsed-tx))

            ;; Done manually for read/write concerns
            (tx-output-0 (default-to {value: u0, scriptPubKey: current-threshold-hashbytes} (element-at tx-outputs u0)))
            (tx-output-1 (default-to {value: u0, scriptPubKey: current-threshold-hashbytes} (element-at tx-outputs u1)))
            (tx-output-2 (default-to {value: u0, scriptPubKey: current-threshold-hashbytes} (element-at tx-outputs u2)))
            (tx-output-3 (default-to {value: u0, scriptPubKey: current-threshold-hashbytes} (element-at tx-outputs u3)))
            (tx-output-4 (default-to {value: u0, scriptPubKey: current-threshold-hashbytes} (element-at tx-outputs u4)))
            (tx-output-5 (default-to {value: u0, scriptPubKey: current-threshold-hashbytes} (element-at tx-outputs u5)))
            (tx-output-6 (default-to {value: u0, scriptPubKey: current-threshold-hashbytes} (element-at tx-outputs u6)))
            (tx-output-7 (default-to {value: u0, scriptPubKey: current-threshold-hashbytes} (element-at tx-outputs u7)))

            ;; versions + hashbytes to scriptPubKey
            (current-unwrapped-threshold-pubkey (unwrap! (contract-call? .sbtc-btc-tx-helper hashbytes-to-scriptpubkey current-threshold-wallet) err-threshold-to-scriptpubkey))
        )

            ;; Assert that transaction was mined...tbd last two params
            (unwrap! (contract-call? .clarity-bitcoin was-segwit-tx-mined-compact burn-height tx header tx-index tree-depth wproof witness-merkle-root witness-reserved-data ctx cproof) err-tx-not-mined)
        
            ;; Assert we're in the transfer window
            (asserts! (is-eq (contract-call? .sbtc-stacking-pool get-current-window) transfer)  err-not-in-transfer-window)

            ;; Assert that balance of previous-threshold-wallet wasn't already transferred
            (asserts! previous-pool-balance-transferred err-balance-already-transferred)

            ;; Assert that every unwrapped receiver addresss is equal to new/current-threshold-wallet
            (asserts! (and 
                (is-eq current-unwrapped-threshold-pubkey (get scriptPubKey tx-output-0))
                (is-eq current-unwrapped-threshold-pubkey (get scriptPubKey tx-output-1))
                (is-eq current-unwrapped-threshold-pubkey (get scriptPubKey tx-output-2))
                (is-eq current-unwrapped-threshold-pubkey (get scriptPubKey tx-output-3))
                (is-eq current-unwrapped-threshold-pubkey (get scriptPubKey tx-output-4))
                (is-eq current-unwrapped-threshold-pubkey (get scriptPubKey tx-output-5))
                (is-eq current-unwrapped-threshold-pubkey (get scriptPubKey tx-output-6))
                (is-eq current-unwrapped-threshold-pubkey (get scriptPubKey tx-output-7))
            ) err-wrong-pubkey)

            ;; Assert that amount transferred > sbtc-registry peg-balance
            (asserts! 
                (<= cycle-peg-balance
                    (+ 
                        (get value tx-output-0)
                        (get value tx-output-1)
                        (get value tx-output-2)
                        (get value tx-output-3)
                        (get value tx-output-4)
                        (get value tx-output-5)
                        (get value tx-output-6)
                        (get value tx-output-7)
                    )
                ) err-peg-balance-not-sufficient)

            ;; Call sbtc-stacking-pool to update balance-transferred value for previous-pool
            (contract-call? .sbtc-stacking-pool balance-was-transferred previous-cycle)
    )
)