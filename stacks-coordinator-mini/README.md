The following Data Flow Diagrams (DFD) illustrate the communication process between DKG signers (Signer 1, Signer 2, ..., Signer N), client UIs, and the Smart Contract in the Stacks blockchain. The first DFD shows the interaction of the signers with the client UIs and the corresponding API to retrieve and sign transactions. The second DFD shows the flow of data between the signers, the smart contract, and the blockchain for key registration, public key retrieval, shared public key computation, message signing, and signature verification.

1. Interaction of the signers with the Client UIs and the API:

```mermaid
graph LR
    A[Signer 1]
    C[Signer 2]
    D[Signer N]
    A -->|Retrieve Unsigned Transactions| I{Signer API}
    C -->|Retrieve Unsigned Transactions| I
    D -->|Retrieve Unsigned Transactions| I
    J([Web or Android Client UI]) -->|Retrieve Unsigned Transactions| I
    J -->|Sign Transaction| I
    I -->|Sign Transaction| A
    I -->|Sign Transaction| C
    I -->|Sign Transaction| D
```

2. Interaction between the signers and the smart contract:

```mermaid
graph LR
    A[Signer 1] -->|Deploy Smart Contract| B{Smart Contract}
    C[Signer 2] -->|Register Public Key| B
    D[Signer N] -->|Register Public Key| B
    B -.->|Store Public Keys in Smart Contract| B
    A -->|Retrieve Public Keys| B
    C -->|Retrieve Public Keys| B
    D -->|Retrieve Public Keys| B
    A -.->|Compute Shared Public Key| A
    C -.->|Compute Shared Public Key| C
    D -.->|Compute Shared Public Key| D
    A -.->|Sign Message| A
    C -.->|Sign Message| C
    D -.->|Sign Message| D
    A -->|Retrieve Coordinator Public Key| B
    C -->|Retrieve Coordinator Public Key| B
    D -->|Retrieve Coordinator Public Key| B
    A -->|Signed Share| E[Coordinator Signer]
    C -->|Signed Share| E
    D -->|Signed Share| E
    E -.->|Compute Signed Transaction| E
    E -->|Broadcast Verified Stacks Transaction| G[Stacks Blockchain]
    E -->|Broadcast Verified Bitcoin Transaction| H[Bitcoin Blockchain]
```