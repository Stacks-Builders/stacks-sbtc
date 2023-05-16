The following sequence diagrams illustrate the communication process between DKG signers (Signer 1, Signer 2, ..., Signer N) and the Smart Contract in the Stacks blockchain. It shows the sequence of events and flow of data between the signers, the smart contract, and the blockchain for key registration, public key retrieval, shared public key computation, message signing, and signature verification. 

```mermaid
sequenceDiagram
participant Signer1
participant SignerN
participant SmartContract
participant Blockchain

Note over Signer1, Blockchain: Key Generation
Signer1->>SmartContract: Register Public Key 1
SignerN->>SmartContract: Register Public Key N

Note over Signer1, Blockchain: Retrieve Transactions
Signer1->>Blockchain: Retrieve Unsigned Transactions
Blockchain-->>Signer1: Unsigned Transactions
SignerN->>Blockchain: Retrieve Unsigned Transactions
Blockchain-->>SignerN: Unsigned Transactions

Note over Signer1, Blockchain: Signing Transactions
loop for each Pending Transaction
    Signer1->>Signer1: Create Partial Signature 1
    SignerN->>SignerN: Create Partial Signature N
    Signer1->>SignerN: Share Partial Signature 1
    SignerN->>Signer1: Share Partial Signature N
    Signer1->>SmartContract: Check Partial Signature N
    SignerN->>SmartContract: Check Partial Signature 1
    Signer1->>Signer1: Combine Partial Signatures 1..N
    SignerN->>SignerN: Combine Partial Signatures 1..N
    Signer1->>Signer1: Sign Transaction
    SignerN->>SignerN: Sign Transaction

    Note over Signer1, Blockchain: Broadcast Transaction
    Signer1->>Blockchain: Broadcast Signed Transaction
    Blockchain-->>Signer1: Transaction Confirmation
    SignerN->>Blockchain: Broadcast Signed Transaction
    Blockchain-->>SignerN: Transaction Confirmation
end
```

This diagram shows the sequence of events that occur when a client activates an autosigner. The AutoSigner API retrieves pending transactions from the blockchain and loops through each transaction. For each transaction, it asks the Signer to determine whether to approve or reject it based on the signer's configuration. If the Signer can determine the decision, it signs the transaction and the AutoSigner API broadcasts it to the blockchain. If the Signer cannot determine the decision, the AutoSigner API notifies the client UI about the decision failure.

```mermaid
sequenceDiagram
participant Blockchain
participant Signer
participant AutoSignerAPI
participant ClientUI

Note over Blockchain, ClientUI: Start Signer

ClientUI->>AutoSignerAPI: Activate Auto Signer
AutoSignerAPI->>Signer: Activate Auto Signer
Signer-->>AutoSignerAPI: Acknowledge Activation
AutoSignerAPI-->>ClientUI: Acknowledge Activation
Signer->>Blockchain: Retrieve Transactions
Blockchain-->>Signer: List of Transactions

Note over Blockchain, ClientUI: Sign Transactions  
loop for each Pending Transaction
    Signer->>Signer: Determine Approval/Rejection
    alt Approval/Rejection Determined
        Signer->>Signer: Sign Transaction
        Signer->>Blockchain: Broadcast Signed Transaction
        Blockchain-->>Signer: Acknowledge Broadcast
    else Cannot Determine
        Note over Blockchain, ClientUI: Notify Client UI
        Signer->>AutoSignerAPI: Notify Decision Failure
        AutoSignerAPI-->>Signer: Acknowledge Notification
        AutoSignerAPI->>ClientUI: Notify Decision Failure
        ClientUI-->>AutoSignerAPI: Acknowledge Notification
    end
end
```


The following sequence diagram shows how a signer server app initializes and interacts with a signer lib, and how it registers and responds to requests via a signer API. The server app relies on the signer lib for cryptographic functions like signing and verifying, while the signer API provides a way for external clients such as a Web Client to interact with the signer server app.

```mermaid
sequenceDiagram
participant SignerServerApp
participant SignerLib
participant SignerAPI
participant WebClient

SignerServerApp->>SignerLib: Initialize Signer Lib
SignerLib-->>SignerServerApp: Signer Lib Initialized

Note over SignerServerApp: Signer Server App Starts

SignerServerApp->>SignerAPI: Register Signer API Endpoints
SignerAPI-->>SignerServerApp: Signer API Endpoints Registered

Note over SignerServerApp: Wait for API Requests

WebClient->>SignerAPI: API Request (e.g., /sign, /verify)
SignerAPI->>SignerServerApp: Incoming API Request
SignerServerApp->>SignerLib: Call Signer Function (e.g., sign, verify)
SignerLib-->>SignerServerApp: Function Result

SignerServerApp->>SignerAPI: API Response
SignerAPI->>WebClient: Response Sent to WebClient
```