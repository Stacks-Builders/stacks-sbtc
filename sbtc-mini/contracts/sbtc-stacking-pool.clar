;; sbtc-mini-stacker-pool
;; pool contract for STX stackers volunteering to maintain the sbtc-mini peg mechanism

;; stacker pool lifecycle
;; 1. principal calls into .pox-2 (allow-contract-caller ...) to signal that this pool contract is given delegation rights
;; 2. principal calls into this contract (delegate-stx ...) to delegate STX to this contract, this does *not* lock the STX but allows delegate (this contract) to issue stacking lock, needs to only be called once
;; 3. any principal calls into this contract (delegate-stack-stx ...) to lock & stack for next cycle

;; voting lifecycle
;; 1. When sbtc-mini-controller signals that voting period is active, stackers call (vote ...) to submit and/or vote on a shared derived wallet address


;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;; Cons, Vars & Maps ;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

;;;;;;;;;;;;;;;;;
;;; constants ;;;
;;;;;;;;;;;;;;;;;
;; state as "normal" suggesting that the pool is operating as expected / wasn't in a "bad state"
(define-constant normal-cycle-len u2016)
(define-constant normal-voting-period-len u300)
(define-constant normal-transfer-period-len u100)
(define-constant normal-penalty-period-len u100)

;; Same burnchain and PoX constants as mainnet
(define-data-var first-burn-block-height uint u666050)
(define-data-var reward-cycle-len uint u2100)
;; Relative burnchain block heights (between 0 and 2100) as to when the system transitions into different states
(define-data-var registration-window-rel-end uint u1600)
(define-data-var voting-window-rel-end uint u1900)
(define-data-var transfer-window-rel-end uint u2000)
(define-data-var penalty-window-rel-end uint u2100)

;;;;;;;;;;;;;;
;;; errors ;;;
;;;;;;;;;;;;;;
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

;;;;;;;;;;;;;;;;;
;;; variables ;;;
;;;;;;;;;;;;;;;;;

;; Highest reward cycle in which all rewards are disbursed (aka the last "good state" peg cycle
(define-data-var last-disbursed-burn-height uint u0)

;; Current cycle threshold wallet
(define-data-var threshold-wallet { version: (buff 1), hashbytes: (buff 32) } { version: 0x00, hashbytes: 0x00 })

;; Current signer minimal
(define-data-var signer-minimal uint u0)

;;;;;;;;;;;;
;;; maps ;;;
;;;;;;;;;;;;

;; Map that tracks all relevant stacker data for a given pool (by cycle index)
(define-map pool uint {
    stackers: (list 100 principal),
    stacked: uint,
    threshold-wallet-candidates: (list 100 { version: (buff 1), hashbytes: (buff 32) }),
    threshold-wallet: (optional { version: (buff 1), hashbytes: (buff 32) }),
    last-aggregation: (optional uint),
    reward-index: (optional uint)
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



;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;; Read-Only Functions ;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

;; Get current cycle pool
(define-read-only (get-current-cycle-pool) 
    (let 
        (
            (current-cycle (contract-call? 'SP000000000000000000002Q6VF78.pox-2 current-pox-reward-cycle))
        )
        (map-get? pool current-cycle)
    )
)

;; Get current window
(define-read-only (get-current-window)
    (let 
        (
            ;; to-do -> (get-peg-state) from .sbtc-controller, returns bool
            (peg-state true)
            (current-cycle (contract-call? 'SP000000000000000000002Q6VF78.pox-2 current-pox-reward-cycle))
            (current-cycle-burn-height (contract-call? 'SP000000000000000000002Q6VF78.pox-2 reward-cycle-to-burn-height current-cycle))
            (next-cycle (contract-call? 'SP000000000000000000002Q6VF78.pox-2 current-pox-reward-cycle))
            (next-cycle-burn-height (contract-call? 'SP000000000000000000002Q6VF78.pox-2 reward-cycle-to-burn-height next-cycle))
            (latest-disbursed-burn-height (var-get last-disbursed-burn-height))
            (start-voting-window (- next-cycle-burn-height (+ normal-voting-period-len normal-transfer-period-len normal-penalty-period-len)))
            (start-transfer-window (- next-cycle-burn-height (+ normal-transfer-period-len normal-penalty-period-len)))
            (start-penalty-window (- next-cycle-burn-height normal-penalty-period-len))
        )

        (if peg-state
            (if (< latest-disbursed-burn-height burn-block-height)
                (if (and (> burn-block-height latest-disbursed-burn-height) (< burn-block-height start-voting-window))
                    "registration"
                    (if (and (>= burn-block-height start-voting-window) (< burn-block-height start-transfer-window))
                        "voting"
                        (if (and (>= burn-block-height start-transfer-window) (< burn-block-height start-penalty-window))
                            "transfer"
                            "penalty"
                        )
                    )
                )
                "disbursement"
            )
            "bad-peg"
        )
    )
)



;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;; Registration Functions ;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

;; Pre-register signer
;; @desc: pre-registers a stacker for the cycle, goal of this function is to gurantee the amount of STX to be stacked for the next cycle
(define-public (signer-pre-register (new-signer principal) (amount-ustx uint) (pox-addr { version: (buff 1), hashbytes: (buff 32)}))
    (let 
        (
            (signer-account (stx-account tx-sender))
            (signer-unlocked-balance (get unlocked signer-account))
            (signer-allowance-status (unwrap! (contract-call? 'ST000000000000000000002AMW42H.pox-2 get-allowance-contract-callers tx-sender (as-contract tx-sender)) err-allowance-not-set))
            (signer-allowance-end-height (get until-burn-ht signer-allowance-status))
            (current-cycle (contract-call? 'SP000000000000000000002Q6VF78.pox-2 current-pox-reward-cycle))
            (next-cycle (+ current-cycle u1))
            (current-pre-signer (map-get? pre-signer {stacker: tx-sender, pool: current-cycle}))
            (current-signer (map-get? signer {stacker: tx-sender, pool: current-cycle}))
        )

        ;; To-do
        ;; Assert that amount-ustx is greater than signer-minimal

        ;; Assert signer-allowance-end-height is either none or block-height is less than signer-allowance-end-height
        (asserts! (or 
            (is-none signer-allowance-end-height) 
            (< burn-block-height (default-to burn-block-height signer-allowance-end-height))
        ) err-allowance-height)

        ;; Assert not already pre-signer or signer
        (asserts! (or (is-none current-pre-signer) (is-none current-signer)) err-already-pre-signer-or-signer)

        ;; Assert we're in the registration window
        (asserts! (is-eq (get-current-window) "registration")  err-not-in-registration-window)

        ;; Delegate-stx to their PoX address
        (unwrap! (contract-call? 'ST000000000000000000002AMW42H.pox-2 delegate-stx amount-ustx (as-contract tx-sender) (some burn-block-height) (some pox-addr)) err-pre-registration-delegate-stx)

        ;; Delegate-stack-stx for next cycle
        (unwrap! (as-contract (contract-call? 'ST000000000000000000002AMW42H.pox-2 delegate-stack-stx new-signer amount-ustx pox-addr burn-block-height u1)) err-pre-registration-delegate-stack-stx)

        ;; Stack aggregate-commit
        ;; As pointed out by Friedger, this fails when the user is already stacking. Match err-branch takes care of this with stack-delegate-increase instead.
        ;;(unwrap! (as-contract (contract-call? 'ST000000000000000000002AMW42H.pox-2 stack-aggregation-commit-indexed pox-addr next-cycle)) err-pre-registration-aggregate-commit)
        (match (as-contract (contract-call? 'ST000000000000000000002AMW42H.pox-2 stack-aggregation-commit-indexed pox-addr next-cycle))
            ok-branch
                true
            err-branch
                (begin

                    ;; Assert stacker isn't attempting to decrease 
                    (asserts! (>= amount-ustx (get locked signer-account)) err-decrease-forbidden)

                    ;; Delegate-stack-increase for next cycle so that there is no cooldown
                    (unwrap! (contract-call? 'SP000000000000000000002Q6VF78.pox-2 delegate-stack-increase new-signer pox-addr (- amount-ustx (get locked signer-account))) err-pre-registration-stack-increase)
                    true
                )
        )

        ;; Map set signer, since this is pre-register that's *not* included in threshold-wallet for next cycle, do not set map for "signer" as this would include them in vote
        ;;(map-set signer {stacker: principal, pool: })

        ;; Record pre-signer
        (ok (map-set pre-signer {stacker: tx-sender, pool: next-cycle} true))

    )
)

;; Register as a signer
;; @desc: registers a signer for the cycle, goal of this function is to gurantee the amount of STX to be stacked for the next cycle
(define-public (signer-register (pre-registered-signer principal) (amount-ustx uint) (pox-addr { version: (buff 1), hashbytes: (buff 32)}) (public-key (buff 33)))
    (let 
        (
            (signer-account (stx-account pre-registered-signer))
            (signer-unlocked-balance (get unlocked signer-account))
            (signer-allowance-status (unwrap! (contract-call? 'ST000000000000000000002AMW42H.pox-2 get-allowance-contract-callers pre-registered-signer (as-contract tx-sender)) err-allowance-not-set))
            (signer-allowance-end-height (get until-burn-ht signer-allowance-status))
            (current-cycle (contract-call? 'SP000000000000000000002Q6VF78.pox-2 current-pox-reward-cycle))
            (next-cycle (+ current-cycle u1))
            (current-pre-signer (map-get? pre-signer {stacker: pre-registered-signer, pool: current-cycle}))
            (current-signer (map-get? signer {stacker: pre-registered-signer, pool: current-cycle}))
            (pox-address-cycle-use (map-get? payout-address-in-cycle pox-addr))
        )

        ;; Assert signer-allowance-end-height is either none or block-height is less than signer-allowance-end-height
        (asserts! (or (is-none signer-allowance-end-height) (< burn-block-height (default-to burn-block-height signer-allowance-end-height))) err-allowance-height)

        ;; Assert we're in a good-peg state & in the registration window
        (asserts! (is-eq (get-current-window) "registration")  err-not-in-registration-window)

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
        (asserts! (is-eq next-cycle (contract-call? 'ST000000000000000000002AMW42H.pox-2 burn-height-to-reward-cycle (get unlock-height signer-account))) err-wont-unlock)

        ;; to-dos
        ;; update all relevant maps

        (ok true)
    )
)



;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;; Voting Functions ;;;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

;; Voting function for deciding the threshold-wallet/PoX address for the next pool & cycle
;; Once a single wallet-candidate reaches 70% of the vote, stack-aggregate-index
(define-public (vote-for-threshold-wallet-candidate (pox-addr { version: (buff 1), hashbytes: (buff 32)}))
    (let 
        (
            (current-cycle (contract-call? 'SP000000000000000000002Q6VF78.pox-2 current-pox-reward-cycle))
            (next-cycle (+ current-cycle u1))
            (current-candidate-status (map-get? votes-per-cycle {cycle: next-cycle, wallet-candidate: pox-addr}))
            (next-pool (unwrap! (map-get? pool next-cycle) err-pool-cycle))
            (next-threshold-wallet (get threshold-wallet next-pool))
            (next-pool-total-stacked (get stacked next-pool))
            (next-pool-signer (unwrap! (map-get? signer {stacker: tx-sender, pool: next-cycle}) err-not-signer))
            (next-pool-signer-amount (get amount next-pool-signer))
        )

        ;; Assert we're in a good-peg state
        (asserts! (contract-call? .sbtc-controller current-peg-state) err-not-in-good-peg-state)

        ;; Assert we're in the voting window
        (asserts! (is-eq (get-current-window) "voting") err-voting-period-closed)

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
                    {threshold-wallet-candidates: (unwrap! (as-max-len? (append (get threshold-wallet-candidates next-pool) pox-addr) u10) err-too-many-candidates)}
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
                (if (>= (/ (* new-candidate-votes u100) next-pool-total-stacked) u70)
                    ;; 70% consensus reached, ready to set next cycle threshold-wallet & attempt to aggregate-commit-index
                    (match (as-contract (contract-call? 'SP000000000000000000002Q6VF78.pox-2 stack-aggregation-commit-indexed pox-addr next-cycle))
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



;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;;;; Private Functions ;;;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

