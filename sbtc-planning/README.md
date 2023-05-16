# sBTC action plan
This document aims to gather a high-level view of the projects we're working on to deliver sBTC
on a best-effort basis.

# Projects in motion

```mermaid
gantt
    dateFormat  YYYY-MM-DD
    title sBTC Action Plan

    section P-01
        Software deployment & delivery     :a1, 2023-04-03, 28d

    section P-02
        Network message signature format              :b1, 2023-04-03, 3d
        Coordinator & signer uses signature format    :b2, after b1, 7d
        sBTC contract interaction                     :b3, after b2, 7d
        Encrypted local store for private keys        :b4, after  b2, 7d

    section P-03
        Design doc and planning                         :c1, 2023-04-03, 14d
        Stacks understanding reveal transactions        :c2, after c1, 14d
        Reference revealer                              :c3, after c2, 14d
        Reference committer                             :c4, after c3, 14d
        End-to-end integration tests                    :c5, after c4, 14d
        sBTC mini integration                           :c6, after c5, 14d

    section P-04
        Support hand-off in stacks node          :d1, after c1, 14d
        Integrate hand-off in sBTC coordinator   :d2, after d1, 14d

    section P-05
        Investigate current state     :e1, 2023-04-03, 7d
        Finalize database schema      :e2, after e1, 7d
        Implement prototype           :e3, after e2, 7d
        Prototype testing             :e4, after e3, 2d
        User interface                :e5, after e3, 7d
        Beta version launch           :e6, after e5, 3d
        Feedback incorporation        :e7, after e6, 14d
        Launch 1.0                    :e8, after e7, 7d

    section P-08
        Risk identification      :f1, 2023-04-03, 14d

    section P-10
        Requirement identification                      :h1, 2023-04-03, 4d
        Deploy working 2.1 testnet                      :h2, after h1, 3d
        Upgrade testnet to 3.0 consensus rules          :h3, after h2, 7d
        Set up globally accessible url for the testnet  :h4, after h2, 1d
        Test mining                                     :h5, after h4, 7d

    section P-24
        Build Stacks Transactions              :i1, 2023-04-03, 7d
        Broadcast Stacks Transactions          :i2, after i1, 1d
        Build Bitcoin Transactions             :i3, 2023-04-06, 1d
        Broadcast Bitcoin Transactions         :i4, after i3, 1d
        Retrieve Config Options from Contract  :i5, 2023-04-06, 2d
        Integration testing     :i6, after i2, 7d

    section P-XX
        Peg-Out Clarity Contract for Request and Fullfilment 157  :j1, 2023-04-17, 14d
        Asset Clarity Contract 279 :j2, 2023-04-17, 14d
        Add segwit to Bitcoin Clarity Library 253 :j3, 2023-04-17, 14d

    section P-XX2
        Peg Transfer Clarity Contract 161 :k1, after j3, 14d
        Controller Clarity Contract 153 :k2, after j3, 14d
        Peg-In Clarity Smart Contract and API 156 :k3, after j3, 14d
        Clarity Decentralized Stacking Pool 278 :k4, after j3, 14d
        Format Clarity to have 1 Line per Exec Branch 305 :k5, after j3, 14d
```


## P-01 Software deployment & delivery
Owner: Sergey Shandar

DoD: We have clear and easy-to-use infrastructure that allows anyone to easily participate in sBTC as a signer.

Project Issue: [CoreEng228](https://github.com/Trust-Machines/core-eng/issues/228)

## P-02 Sign and validate FROST shares
Owner: Joey Yandle

DoD: Our FROST implementation only accepts signed messages.

Project Issue: [CoreEng229](https://github.com/Trust-Machines/core-eng/issues/229)

## P-03 Commit-reveal peg operations
Owner: Mårten Blankfors

DoD: sBTC burnchain operations support a format usable from custodian wallets.

Project Issue: [CoreEng230](https://github.com/Trust-Machines/core-eng/issues/230)

## P-04 Peg-handoff system
Owner: Mårten Blankfors

DoD: We have a secure mechanism to hand over custody of the sBTC peg wallet.

Project Issue: [CoreEng231](https://github.com/Trust-Machines/core-eng/issues/231)

## P-05 Stacker DB
Owner: Stjepan Golemac

DoD: Stacks nodes support hosting auxiliary smart contract data which can be used to support stacker communication in sBTC.

Project Issue: [CoreEng232](https://github.com/Trust-Machines/core-eng/issues/232)

## P-08 Risk identification for mini sBTC
Owner: José Orlicki

DoD: Risks revolving mini sBTC have been identified and addressed.

Project Issue: [CoreEng233](https://github.com/Trust-Machines/core-eng/issues/233)

## P-10 Testnet network
Owner: Sayak Chatterjee

DoD: There is a private testnet network that runs the Nakamoto consensus and is equipped with testnet sBTC mechanisms.

Project Issue: [CoreEng234](https://github.com/Trust-Machines/core-eng/issues/234)

## P-24 Alpha coordinator
Owner: Jacinta Ferrant

DoD: Stacks signers can coordinate to fulfill their responsibilities as needed for sBTC alpha.

Project Issue: [CoreEng235](https://github.com/Trust-Machines/core-eng/issues/235)
