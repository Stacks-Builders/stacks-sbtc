-- Add migration script here
CREATE TABLE IF NOT EXISTS sbtc_signers (
        signer_id INTEGER NOT NULL,
        user_id INTEGER NOT NULL,
        status TEXT NOT NULL,

        PRIMARY KEY(signer_id, user_id)
    );

CREATE TABLE IF NOT EXISTS keys (
            key TEXT NOT NULL,
            signer_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
    
            PRIMARY KEY(key, signer_id, user_id),
            FOREIGN KEY(signer_id, user_id) REFERENCES sbtc_signers(signer_id, user_id)
        );