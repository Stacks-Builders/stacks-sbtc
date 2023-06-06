;; sbtc-mini-stacker-pool


;;;;; Cons, Vars & Maps ;;;;;

;;; constants ;;;
;; cycle windows
(define-constant disbursement 0x00)
(define-constant registration 0x01)
(define-constant voting 0x02)
(define-constant transfer 0x03)
(define-constant penalty 0x04)
(define-constant bad-peg-state 0x05)

;; state as "normal" suggesting that the pool is operating as expected / wasn't in a "bad state"
(define-constant normal-cycle-len u2016)
(define-constant normal-voting-period-len u300)
(define-constant normal-transfer-period-len u100)
(define-constant normal-penalty-period-len u100)

;; Burn POX address for penalizing stackers/signers
(define-constant pox-burn-address { version: 0x00, hashbytes: 0x0000000000000000000000000000000000000000000000000000000000000000})

;; Dust limit placeholder for checking that pox-rewards were disbursed (in sats)
(define-constant dust-limit u100)

;;; errors ;;;
(define-constant err-not-signer (err u0))
(define-constant err-allowance-not-set (err u1))
(define-constant err-allowance-height (err u2))
(define-constant err-already-pre-signer-or-signer (err u3))
(define-constant err-not-in-registration-window (err u4))
(define-constant err-pre-registration-delegate-stx (err u5))
(define-constant err-pre-registration-delegate-stack-stx (err u6))
(define-constant err-pre-registration-aggregate-commit (err u7))
(define-constant err-public-key-already-used (err u8))
(define-constant err-pox-address-re-use (err u9))
(define-constant err-not-enough-stacked (err u10))
(define-constant err-wont-unlock (err u11))
(define-constant err-voting-period-closed (err u12))
(define-constant err-already-voted (err u13))
(define-constant err-decrease-forbidden (err u14))
(define-constant err-pre-registration-stack-increase (err u15))
(define-constant err-not-in-good-peg-state (err u16))
(define-constant err-unwrapping-candidate (err u17))
(define-constant err-pool-cycle (err u18))
(define-constant err-too-many-candidates (err u19))
(define-constant err-not-in-transfer-window (err u20))
(define-constant err-unhandled-request (err u21))
(define-constant err-invalid-penalty-type (err u22))
(define-constant err-already-disbursed (err u23))
(define-constant err-not-handoff-contract (err u24))
(define-constant err-parsing-btc-tx (err u25))
(define-constant err-threshold-wallet-is-none (err u26))
(define-constant err-tx-not-mined (err u27))
(define-constant err-wrong-pubkey (err u28))
(define-constant err-dust-remains (err u29))
(define-constant err-balance-not-transferred (err u30))
(define-constant err-not-in-penalty-window (err u31))
(define-constant err-rewards-already-disbursed (err u32))
(define-constant err-not-in-voting-window (err u33))
(define-constant err-set-peg-state (err u34))
(define-constant err-not-protocol-caller (err u35))
(define-constant err-threshold-percent-out-of-range (err u36))


;;; variables ;;;

;; Threshold consensus (in 3 digit %)
(define-data-var threshold-consensus uint u700)

;; Highest reward cycle in which all rewards are disbursed (aka the last "good state" peg cycle
(define-data-var last-disbursed-burn-height uint u0)

;; Current cycle threshold wallet
(define-data-var threshold-wallet { version: (buff 1), hashbytes: (buff 32) } { version: 0x00, hashbytes: 0x00 })

;; Current signer minimal
(define-data-var signer-minimal uint u0)

;; Same burnchain and PoX constants as mainnet
(define-constant first-burn-block-height u666050)
(define-data-var reward-cycle-len uint u2100)

;; Relative burnchain block heights (between 0 and 2100) as to when the system transitions into different states
(define-data-var registration-window-rel-end uint u1600)
(define-data-var voting-window-rel-end uint u1900)
(define-data-var transfer-window-rel-end uint u2000)
(define-data-var penalty-window-rel-end uint u2100)

;;; maps ;;;

;; Map that tracks all relevant stacker data for a given pool (by cycle index)
(define-map pool uint {
    stackers: (list 100 principal),
    stacked: uint,
    threshold-wallet-candidates: (list 100 { version: (buff 1), hashbytes: (buff 32) }),
    threshold-wallet: (optional { version: (buff 1), hashbytes: (buff 32) }),
    last-aggregation: (optional uint),
    reward-index: (optional uint),
    balance-transferred: bool,
    rewards-disbursed: bool
})

;; Map that tracks all stacker/signer data for a given principal & pool (by cycle index)
(define-map signer {stacker: principal, pool: uint} {
    amount: uint,
    ;; pox-addrs must be unique per cycle
    pox-addr: { version: (buff 1), hashbytes: (buff 32) },
    vote: (optional { version: (buff 1), hashbytes: (buff 32) }),
    public-key: (buff 33),
    lock-period: uint,
    btc-earned: (optional uint)
})

;; Map that tracks all votes per cycle
(define-map votes-per-cycle {cycle: uint, wallet-candidate: { version: (buff 1), hashbytes: (buff 32) } } {
    aggregate-commit-index: (optional uint),
    votes-in-ustx: uint,
    num-signer: uint,
})

;; Map that tracks all pre-signer stacker/signer sign-ups for a given principal & pool (by cycle index)
(define-map pre-signer {stacker: principal, pool: uint} bool)

;; Map of reward cycle to pox reward set index.
(define-map pox-addr-indices uint uint)

;; Map of reward cyle to block height of last commit
(define-map last-aggregation uint uint)

;; Allowed contract-callers handling a user's stacking activity.
(define-map allowance-contract-callers { sender: principal, contract-caller: principal} {
    until-burn-ht: (optional uint)
})

;; All public keys (buff 33) ever used
(define-map public-keys-used (buff 33) bool)

;; All PoX addresses used per reward cycle (so we don't re-use them)
(define-map payout-address-in-cycle { version: (buff 1), hashbytes: (buff 32) } uint)


;;;;; Read-Only Functions ;;;;;

;; Check if caller is a protocol caller
(define-read-only (is-protocol-caller (who principal))
	(contract-call? .sbtc-controller is-protocol-caller contract-caller)
)

;; Get current cycle pool
(define-read-only (get-current-cycle-pool) 
    (let 
        (
            (current-cycle (contract-call? .pox-3 current-pox-reward-cycle))
        )
        (map-get? pool current-cycle)
    )
)

;; Get specific cycle pool
(define-read-only (get-specific-cycle-pool (specific-cycle uint)) 
        (map-get? pool specific-cycle)
)

;; Get signer in cycle
(define-read-only (get-signer-in-cycle (signer-principal principal) (cycle uint))
    (let 
        (
            (current-signer (default-to {
                amount: u0,
                pox-addr: { version: 0x00, hashbytes: 0x00 },
                vote: none,
                public-key: 0x00,
                lock-period: u0,
                btc-earned: none
            } (map-get? signer {stacker: signer-principal, pool: cycle})))
        )
        (ok (get public-key current-signer))
    )
)

;; Get current window
(define-read-only (get-current-window)
    (let 
        (
            (peg-state (contract-call? .sbtc-registry current-peg-state))
            (current-cycle (contract-call? .pox-3 current-pox-reward-cycle))
            (current-cycle-burn-height (contract-call? .pox-3 reward-cycle-to-burn-height current-cycle))
            (next-cycle (contract-call? .pox-3 current-pox-reward-cycle))
            (next-cycle-burn-height (contract-call? .pox-3 reward-cycle-to-burn-height next-cycle))
            (latest-disbursed-burn-height (var-get last-disbursed-burn-height))
            (start-voting-window (- next-cycle-burn-height (+ normal-voting-period-len normal-transfer-period-len normal-penalty-period-len)))
            (start-transfer-window (- next-cycle-burn-height (+ normal-transfer-period-len normal-penalty-period-len)))
            (start-penalty-window (- next-cycle-burn-height normal-penalty-period-len))
        )

        ;;(asserts! peg-state bad-peg-state)

        ;;(asserts! (< latest-disbursed-burn-height burn-block-height) disbursement)

        ;;(asserts! (and (> burn-block-height latest-disbursed-burn-height) (< burn-block-height start-voting-window)) registration)

        ;;(asserts! (and (>= burn-block-height start-voting-window) (< burn-block-height start-transfer-window)) voting)

        ;;(asserts! (and (>= burn-block-height start-transfer-window) (< burn-block-height start-penalty-window)) transfer)

        ;;(asserts! (>= burn-block-height start-penalty-window) penalty)
        penalty

    )
)



;;;;;;; Disbursement Functions ;;;;;;;
;; Function that proves POX rewards have been disbursed from the previous threshold wallet to the previous pool signers. This happens in x steps:
;; 1. Fetch previous pool threshold-wallet / pox-reward-address
;; 2. Parse-tx to get all (8) outputs
;; 3. Check that for each output, the public-key matches the pox-reward-address
;; 4. Check that for each output, the amount is lower than constant dust 
;; Note - this may be updated to later check against a specific balance

;; Disburse function for signers in (n - 1) to verify that their pox-rewards have been disbursed
(define-public (prove-rewards-were-disbursed 
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
            (current-cycle (contract-call? .pox-3 burn-height-to-reward-cycle block-height))
            (previous-cycle (- current-cycle u1))
            (previous-pool (unwrap! (map-get? pool previous-cycle) err-pool-cycle))
            (unwrapped-previous-threshold-wallet (unwrap! (get threshold-wallet previous-pool) err-threshold-wallet-is-none))
            (previous-threshold-wallet (get hashbytes unwrapped-previous-threshold-wallet))
            (previous-threshold-wallet-version (get version unwrapped-previous-threshold-wallet))
            (previous-pool-disbursed (get rewards-disbursed previous-pool))
            (parsed-tx (unwrap! (contract-call? .clarity-bitcoin parse-tx tx) err-parsing-btc-tx))
            (tx-outputs (get outs parsed-tx))
            ;; Done manually for read/write concerns
            (tx-output-0 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u0)))
            (tx-output-1 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u1)))
            (tx-output-2 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u2)))
            (tx-output-3 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u3)))
            (tx-output-4 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u4)))
            (tx-output-5 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u5)))
            (tx-output-6 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u6)))
            (tx-output-7 (default-to {value: u0, scriptPubKey: previous-threshold-wallet} (element-at tx-outputs u7)))
        )
        
            ;; Assert we're in the disbursement window
            (asserts! (is-eq (get-current-window) disbursement)  err-not-in-registration-window)

            ;; Assert that balance of previous-threshold-wallet was transferred
            (asserts! (get balance-transferred previous-pool) err-balance-not-transferred)

            ;; Assert that rewards haven't already been disbursed
            (asserts! (not previous-pool-disbursed) err-already-disbursed)

            ;; Assert that transaction was mined...tbd last two params
            (unwrap! (contract-call? .clarity-bitcoin was-segwit-tx-mined-compact burn-height tx header tx-index tree-depth wproof 0x 0x ctx cproof) err-tx-not-mined)

            ;; Assert that every unwrapped receiver addresss is equal to previous-threshold-wallet
            (asserts! (and 
                (is-eq previous-threshold-wallet (get scriptPubKey tx-output-0))
                (is-eq previous-threshold-wallet (get scriptPubKey tx-output-1))
                (is-eq previous-threshold-wallet (get scriptPubKey tx-output-2))
                (is-eq previous-threshold-wallet (get scriptPubKey tx-output-3))
                (is-eq previous-threshold-wallet (get scriptPubKey tx-output-4))
                (is-eq previous-threshold-wallet (get scriptPubKey tx-output-5))
                (is-eq previous-threshold-wallet (get scriptPubKey tx-output-6))
                (is-eq previous-threshold-wallet (get scriptPubKey tx-output-7))
            ) err-wrong-pubkey)

            ;; Assert that every unwrapped output is less than dust
            (asserts! (and 
                (< (get value tx-output-0) dust-limit)
                (< (get value tx-output-1) dust-limit)
                (< (get value tx-output-2) dust-limit)
                (< (get value tx-output-3) dust-limit)
                (< (get value tx-output-4) dust-limit)
                (< (get value tx-output-5) dust-limit)
                (< (get value tx-output-6) dust-limit)
                (< (get value tx-output-7) dust-limit)
            ) err-dust-remains)

            ;; All POX rewards have been distributed, update relevant vars/maps
            (var-set last-disbursed-burn-height block-height)
            (ok (map-set pool previous-cycle (merge 
                previous-pool 
                {rewards-disbursed: true}
            )))
    )
)



;;;;; Registration Functions ;;;;;

;; @desc: pre-registers a stacker for the cycle, goal of this function is to gurantee the amount of STX to be stacked for the next cycle
(define-public (signer-pre-register (new-signer principal) (amount-ustx uint) (pox-addr { version: (buff 1), hashbytes: (buff 32)}))
    (let 
        (
            (signer-account (stx-account tx-sender))
            (signer-unlocked-balance (get unlocked signer-account))
            (signer-allowance-status (unwrap! (contract-call? .pox-3 get-allowance-contract-callers tx-sender (as-contract tx-sender)) err-allowance-not-set))
            (signer-allowance-end-height (get until-burn-ht signer-allowance-status))
            (current-cycle (contract-call? .pox-3 current-pox-reward-cycle))
            (next-cycle (+ current-cycle u1))
            (current-pre-signer (map-get? pre-signer {stacker: tx-sender, pool: current-cycle}))
            (current-signer (map-get? signer {stacker: tx-sender, pool: current-cycle}))
        )

        ;; Assert that amount-ustx is greater than signer-minimal
        (asserts! (>= amount-ustx (var-get signer-minimal)) err-not-enough-stacked)

        ;; Assert signer-allowance-end-height is either none or block-height is less than signer-allowance-end-height
        (asserts! (or 
            (is-none signer-allowance-end-height) 
            (< burn-block-height (default-to burn-block-height signer-allowance-end-height))
        ) err-allowance-height)

        ;; Assert not already pre-signer or signer
        (asserts! (or (is-none current-pre-signer) (is-none current-signer)) err-already-pre-signer-or-signer)

        ;; Assert we're in the registration window
        (asserts! (is-eq (get-current-window) registration)  err-not-in-registration-window)

        ;; Delegate-stx to their PoX address
        (unwrap! (contract-call? .pox-3 delegate-stx amount-ustx (as-contract tx-sender) (some burn-block-height) (some pox-addr)) err-pre-registration-delegate-stx)

        ;; Delegate-stack-stx for next cycle
        (unwrap! (as-contract (contract-call? .pox-3 delegate-stack-stx new-signer amount-ustx pox-addr burn-block-height u1)) err-pre-registration-delegate-stack-stx)

        ;; Stack aggregate-commit
        ;; As pointed out by Friedger, this fails when the user is already stacking. Match err-branch takes care of this with stack-delegate-increase instead.
        (match (as-contract (contract-call? .pox-3 stack-aggregation-commit-indexed pox-addr next-cycle))
            ok-branch
                true
            err-branch
                (begin

                    ;; Assert stacker isn't attempting to decrease 
                    (asserts! (>= amount-ustx (get locked signer-account)) err-decrease-forbidden)

                    ;; Delegate-stack-increase for next cycle so that there is no cooldown
                    (unwrap! (contract-call? .pox-3 delegate-stack-increase new-signer pox-addr (- amount-ustx (get locked signer-account))) err-pre-registration-stack-increase)
                    true
                )
        )

        ;; Record pre-signer
        (ok (map-set pre-signer {stacker: tx-sender, pool: next-cycle} true))

    )
)

;; @desc: registers a signer for the cycle, goal of this function is to gurantee the amount of STX to be stacked for the next cycle
(define-public (signer-register (pre-registered-signer principal) (amount-ustx uint) (pox-addr { version: (buff 1), hashbytes: (buff 32)}) (public-key (buff 33)))
    (let 
        (
            (signer-account (stx-account pre-registered-signer))
            (signer-unlocked-balance (get unlocked signer-account))
            (signer-allowance-status (unwrap! (contract-call? .pox-3 get-allowance-contract-callers pre-registered-signer (as-contract tx-sender)) err-allowance-not-set))
            (signer-allowance-end-height (get until-burn-ht signer-allowance-status))
            (current-cycle (contract-call? .pox-3 current-pox-reward-cycle))
            (next-cycle (+ current-cycle u1))
            (current-pre-signer (map-get? pre-signer {stacker: pre-registered-signer, pool: current-cycle}))
            (current-signer (map-get? signer {stacker: pre-registered-signer, pool: current-cycle}))
            (pox-address-cycle-use (map-get? payout-address-in-cycle pox-addr))
        )

        ;; Assert signer-allowance-end-height is either none or block-height is less than signer-allowance-end-height
        (asserts! (or (is-none signer-allowance-end-height) (< burn-block-height (default-to burn-block-height signer-allowance-end-height))) err-allowance-height)

        ;; Assert we're in a good-peg state & in the registration window
        (asserts! (is-eq (get-current-window) registration)  err-not-in-registration-window)

        ;; Assert the public-key hasn't been used before
        (asserts! (is-none (map-get? public-keys-used public-key)) err-public-key-already-used)
        
        ;; Assert that pox-address-cycle-use is either none or the result is not equal to the next cycle
        (asserts! (or 
            (is-none pox-address-cycle-use) 
            (not (is-eq (default-to u0 pox-address-cycle-use) next-cycle)) 
        ) err-pox-address-re-use)

        ;; Assert that pre-registered-signer is either pre-signed for the current-cycle or is a signer for the current-cycle && voted in the last one (?)
        (asserts! (or 
            (is-some current-pre-signer) 
            (and (is-some current-signer) (is-some (get vote current-signer)))
        ) err-already-pre-signer-or-signer)

        ;; Assert that pre-registered signer has at least the amount of STX to be stacked already locked (entire point of pre-registering)
        (asserts! (>= (get locked signer-account) amount-ustx) err-not-enough-stacked)

        ;; Assert that pre-registered signer will unlock in the next cycle
        (asserts! (is-eq next-cycle (contract-call? .pox-3 burn-height-to-reward-cycle (get unlock-height signer-account))) err-wont-unlock)

        ;; to-dos
        ;; update all relevant maps

        (ok true)
    )
)



;;;;; Voting Functions ;;;;;;;

;; @desc: Voting function for deciding the threshold-wallet/PoX address for the next pool & cycle, once a single wallet-candidate reaches 70% of the vote, stack-aggregate-index
(define-public (vote-for-threshold-wallet-candidate (pox-addr { version: (buff 1), hashbytes: (buff 32)}))
    (let 
        (
            (current-cycle (contract-call? .pox-3 current-pox-reward-cycle))
            (next-cycle (+ current-cycle u1))
            (current-candidate-status (map-get? votes-per-cycle {cycle: next-cycle, wallet-candidate: pox-addr}))
            (next-pool (unwrap! (map-get? pool next-cycle) err-pool-cycle))
            (next-threshold-wallet (get threshold-wallet next-pool))
            (next-pool-total-stacked (get stacked next-pool))
            (next-pool-signer (unwrap! (map-get? signer {stacker: tx-sender, pool: next-cycle}) err-not-signer))
            (next-pool-signer-amount (get amount next-pool-signer))
        )

        ;; Assert we're in a good-peg state
        (asserts! (contract-call? .sbtc-registry current-peg-state) err-not-in-good-peg-state)

        ;; Assert we're in the voting window
        (asserts! (is-eq (get-current-window) voting) err-voting-period-closed)

        ;; Assert signer hasn't voted yet
        (asserts! (is-none (get vote next-pool-signer)) err-already-voted)

        ;; Update signer map with vote
        (map-set signer {stacker: tx-sender, pool: next-cycle} (merge 
            next-pool-signer 
            {vote: (some pox-addr)}
        ))

        ;; Check whether map-entry for candidate-wallet already exists
        (if (is-none current-candidate-status)

            ;; Candidate doesn't exist, update relevant maps: votes-per-cycle & pool
            (begin 
                ;; Update votes-per-cycle map with first vote for this wallet-candidate
                (map-set votes-per-cycle {cycle: next-cycle, wallet-candidate: pox-addr} {
                    aggregate-commit-index: none,
                    votes-in-ustx: (get amount next-pool-signer),
                    num-signer: u1,
                })
                
                ;; Update pool map by appending wallet-candidate to list of candidates
                (map-set pool next-cycle (merge 
                    next-pool
                    {threshold-wallet-candidates: (unwrap! (as-max-len? (append (get threshold-wallet-candidates next-pool) pox-addr) u100) err-too-many-candidates)}
                ))
            )

            ;; Candidate exists, update relevant maps: votes-per-cycle, signer, & check whether this wallet is over threshold
            (let
                (
                    (unwrapped-candidate (unwrap! current-candidate-status err-unwrapping-candidate))
                    (unwrapped-candidate-votes (get votes-in-ustx unwrapped-candidate))
                    (unwrapped-candidate-num-signer (get num-signer unwrapped-candidate))
                    (new-candidate-votes (+ (get amount next-pool-signer) (get votes-in-ustx unwrapped-candidate)))
                )

                ;; Update votes-per-cycle map
                (map-set votes-per-cycle {cycle: next-cycle, wallet-candidate: pox-addr} (merge
                    unwrapped-candidate
                    {
                        votes-in-ustx: (+ next-pool-signer-amount unwrapped-candidate-votes),
                        num-signer: (+ u1 unwrapped-candidate-num-signer)
                    }
                ))

                ;; Update signer map
                (map-set signer {stacker: tx-sender, pool: next-cycle} (merge next-pool-signer { vote: (some pox-addr) }))

                ;; Check if 70% wallet consensus has been reached
                (if (>= (/ (* new-candidate-votes u1000) next-pool-total-stacked) (var-get threshold-consensus))
                    ;; 70% consensus reached, ready to set next cycle threshold-wallet & attempt to aggregate-commit-index
                    (match (as-contract (contract-call? .pox-3 stack-aggregation-commit-indexed pox-addr next-cycle))
                        ;; Okay result, update pool map with last-aggregation (block-height) & reward-index
                        ok-result
                            (map-set pool next-cycle (merge
                                next-pool
                                {last-aggregation: (some block-height), reward-index: (some ok-result)}
                            ))
                        err-result
                            false
                    )
                    ;; 70% consensus not reached, continue
                    false
                )

            )
        )

        (ok true)
    )
)



;;;;;;; Transfer Functions ;;;;;;;

;; Transfer function for proving that current/soon-to-be-old signers have transferred the peg balance to the next threshold-wallet
;; Can only be called by the sbtc-peg-transfer/handoff contract. If successful, balance-disbursed is set to true for the previous pool
(define-public (balance-was-transferred (previous-cycle uint))
    (let 
        (
            (previous-pool (unwrap! (map-get? pool previous-cycle) err-pool-cycle))
        )

            ;; Assert that contract-caller is .sbtc-peg-transfer / handoff contract
            (asserts! (is-eq contract-caller .sbtc-peg-transfer) err-not-handoff-contract)

            ;; peg-transfer /handoff success, update relevant vars/maps
            (ok (map-set pool previous-cycle (merge 
                previous-pool 
                {balance-transferred: true}
            )))
        
    )
)



;;;;;;; Penalty Functions ;;;;;;;

;; Penalty function for an unexpired, unhandled request-post-vote
(define-public (penalty-unhandled-request)
    (let
        (
            (current-cycle (contract-call? .pox-3 current-pox-reward-cycle))
            (next-cycle (+ current-cycle u1))
            (current-pool (unwrap! (map-get? pool current-cycle) err-pool-cycle))
            (current-pool-stackers (get stackers current-pool))
            (next-pool (unwrap! (map-get? pool next-cycle) err-pool-cycle))
        )

        ;; Assert that we're in the transfer window
        (asserts! (is-eq (get-current-window) transfer) err-not-in-transfer-window)

        ;; Assert that pending-wallet-peg-outs is not equal to zero
        (asserts! (> (contract-call? .sbtc-registry get-pending-wallet-peg-outs) u0) err-unhandled-request)

        ;; Penalize stackers by re-stacking but with a pox-reward address of burn address
        (penalize-helper next-cycle next-pool)

        ;; Change peg-state to "bad-peg" :(
        (contract-call? .sbtc-registry set-peg-state false)

    )
)

;; Penalty function for when a new wallet vote threshold (70%) is not met in time
(define-public (penalty-vote-threshold)
    (let
        (
            (current-cycle (contract-call? .pox-3 current-pox-reward-cycle))
            (next-cycle (+ current-cycle u1))
            (current-pool (unwrap! (map-get? pool current-cycle) err-pool-cycle))
            (current-pool-stackers (get stackers current-pool))
            (next-pool (unwrap! (map-get? pool next-cycle) err-pool-cycle))
            (next-pool-threshold-wallet (get threshold-wallet next-pool))
        )

        ;; Assert that we're in the transfer window
        (asserts! (is-eq (get-current-window) transfer) err-not-in-transfer-window)

        ;; Assert that next-pool-threshold-wallet is-none
        (asserts! (is-some next-pool-threshold-wallet) err-unhandled-request)

        ;; Assert that pending-wallet-peg-outs is equal to zero
        (asserts! (is-eq (contract-call? .sbtc-registry get-pending-wallet-peg-outs) u0) err-unhandled-request)

        ;; Penalize stackers by re-stacking but with a pox-reward address of burn address
        (penalize-helper next-cycle next-pool)

        ;; Change peg-state to "bad-peg"
        (contract-call? .sbtc-registry set-peg-state false)

    )
)

;; Penalty function for when a stacker fails to transfer the current peg balance to the next threshold wallet
(define-public (penalty-balance-transfer)
    (let
        (
            (current-cycle (contract-call? .pox-3 current-pox-reward-cycle))
            (current-pool (unwrap! (map-get? pool current-cycle) err-pool-cycle))
            (current-pool-balance-transfer (get balance-transferred current-pool))
            (current-pool-stackers (get stackers current-pool))
            (next-cycle (+ current-cycle u1))
            (next-pool (unwrap! (map-get? pool next-cycle) err-pool-cycle))
            (next-pool-threshold-wallet (get threshold-wallet next-pool))
        )

        ;; Assert that we're in the penalty window
        (asserts! (is-eq (get-current-window) penalty) err-not-in-penalty-window)

        ;; Assert that next-pool-threshold-wallet is-some / was voted in correctly
        (asserts! (is-some next-pool-threshold-wallet) err-unhandled-request)

        ;; Assert that balance-transfer is false (wasn't already transferred)
        (asserts! (not current-pool-balance-transfer) err-unhandled-request)

        ;; Penalize stackers by re-stacking but with a pox-reward address of burn address
        (penalize-helper next-cycle next-pool)

        ;; Change peg-state to "bad-peg"
        (contract-call? .sbtc-registry set-peg-state false)

    )
)

;; Penalty function for when the pox-rewards aren't disbursed in time / registration is missing
(define-public (penalty-pox-reward-disbursement)
    (let
        (
            (current-cycle (contract-call? .pox-3 current-pox-reward-cycle))
            (previous-cycle (- current-cycle u1))
            (previous-pool (unwrap! (map-get? pool previous-cycle) err-pool-cycle))
            (previous-pool-rewards-disbursed (get rewards-disbursed previous-pool))
            (next-cycle (+ current-cycle u1))
            (next-pool (unwrap! (map-get? pool next-cycle) err-pool-cycle))
        )

        ;; Assert that we're in the voting window (aka registration window was missed)
        (asserts! (is-eq (get-current-window) voting) err-not-in-voting-window)

        ;; Assert that last pool disbursed is false (wasn't already disbursed)
        (asserts! (not previous-pool-rewards-disbursed) err-rewards-already-disbursed)

        ;; To review/ask about
        ;; Penalize stackers by re-stacking but with a pox-reward address of burn address
        ;; Distributing pox-rewards punishes current signers even if they did nothing wrong?
        ;; Meanwhile previous signers aren't punished since the pox-rewards are already at the previous threshold wallet
        ;; (match (as-contract (contract-call? .pox-3 stack-aggregation-commit-indexed pox-burn-address next-cycle))
        ;;     ok-result
        ;;         (map-set pool next-cycle (merge
        ;;             next-pool
        ;;             {last-aggregation: (some block-height), reward-index: (some ok-result)}
        ;;         ))
        ;;     err-result
        ;;         false
        ;; )
        (penalize-helper next-cycle next-pool)

        ;; Change peg-state to "bad-peg"
        (contract-call? .sbtc-registry set-peg-state false)

    )
)

;; Penalize helper
(define-private (penalize-helper (next-cycle uint) (next-pool 
        {
            stackers: (list 100 principal),
            stacked: uint,
            threshold-wallet-candidates: (list 100 { version: (buff 1), hashbytes: (buff 32) }),
            threshold-wallet: (optional { version: (buff 1), hashbytes: (buff 32) }),
            last-aggregation: (optional uint),
            reward-index: (optional uint),
            balance-transferred: bool,
            rewards-disbursed: bool
        }
    ))
    (match (as-contract (contract-call? .pox-3 stack-aggregation-commit-indexed pox-burn-address next-cycle))
        ok-result
            (map-set pool next-cycle (merge
                next-pool
                {last-aggregation: (some block-height), reward-index: (some ok-result)}
            ))
        err-result
            false
    )
)


;;; Protocol Functions ;;;;
;; Protocol function for updating threshold-percent
(define-public (update-threshold-percent (new-threshold-percent uint))
    (begin

        ;; Assert that caller is protocol caller
        (unwrap! (is-protocol-caller contract-caller) err-not-protocol-caller)

        ;; Assert that new-threshold-percent is greater u500 or less than u950
        (asserts! (and (>= new-threshold-percent u500) (<= new-threshold-percent u950)) err-threshold-percent-out-of-range)

        ;; Update threshold-percent
        (ok (var-set threshold-consensus new-threshold-percent))
    )
)