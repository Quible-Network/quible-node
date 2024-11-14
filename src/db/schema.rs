use surrealdb::engine::any::Any as AnyDb;
use surrealdb::Surreal;

pub async fn initialize_tracker_db(db: &Surreal<AnyDb>) -> surrealdb::Result<()> {
    db.query("DEFINE TABLE tracker_pings SCHEMAFULL;").await?;
    db.query("DEFINE FIELD peer_id ON tracker_pings TYPE string;")
        .await?;
    db.query("DEFINE FIELD timestamp ON tracker_pings TYPE int;")
        .await?;

    Ok(())
}

pub async fn initialize_db(db: &Surreal<AnyDb>) -> surrealdb::Result<()> {
    // Create table for blocks
    db.query("DEFINE TABLE blocks SCHEMAFULL;").await?;
    db.query("DEFINE FIELD hash ON blocks TYPE string;").await?;
    db.query("DEFINE FIELD height ON blocks TYPE int;").await?;
    db.query("DEFINE FIELD header ON blocks FLEXIBLE TYPE object")
        .await?;
    // db.query("DEFINE FIELD header.data.timestamp ON blocks TYPE datetime;").await?;
    // db.query("DEFINE FIELD header.data.timestamp ON blocks TYPE int;").await?;
    db.query("DEFINE FIELD transactions ON blocks FLEXIBLE TYPE array;")
        .await?;

    // Create table for pending transactions
    db.query("DEFINE TABLE pending_transactions SCHEMAFULL;")
        .await?;
    // db.query("DEFINE FIELD hash ON pending_transactions TYPE bytes;").await?;
    db.query("DEFINE FIELD hash ON pending_transactions TYPE string;")
        .await?;
    db.query("DEFINE FIELD data ON pending_transactions TYPE object;")
        .await?;
    db.query("DEFINE FIELD data.version ON pending_transactions TYPE string;")
        .await?;
    db.query("DEFINE FIELD data.data ON pending_transactions FLEXIBLE TYPE object;")
        .await?;
    // db.query("DEFINE FIELD data.data.inputs ON pending_transactions TYPE array;").await?;
    // db.query("DEFINE FIELD data.data.outputs ON pending_transactions TYPE array;").await?;
    // db.query("DEFINE FIELD data.data.locktime ON pending_transactions TYPE int;").await?;
    db.query("DEFINE FIELD size ON pending_transactions TYPE int;")
        .await?;

    // TODO: define the event type more thoroughly here to avoid the use of FLEXIBLE
    // db.query("DEFINE FIELD data.data.inputs.* ON pending_transactions FLEXIBLE TYPE object;").await?;

    // TODO: define the event type more thoroughly here to avoid the use of FLEXIBLE
    // db.query("DEFINE FIELD data.data.outputs.* ON pending_transactions FLEXIBLE TYPE object;").await?;

    // Create table for author quirkle counts
    db.query("DEFINE TABLE author_quirkle_counts SCHEMAFULL;")
        .await?;
    db.query("DEFINE FIELD author ON author_quirkle_counts TYPE string;")
        .await?;
    db.query("DEFINE FIELD count ON author_quirkle_counts TYPE int;")
        .await?;

    // Create table for quirkle proof TTLs
    db.query("DEFINE TABLE quirkle_proof_ttls SCHEMAFULL;")
        .await?;
    db.query("DEFINE FIELD quirkle_root ON quirkle_proof_ttls TYPE string;")
        .await?;
    db.query("DEFINE FIELD proof_ttl ON quirkle_proof_ttls TYPE int;")
        .await?;

    // Create table for quirkle items
    db.query("DEFINE TABLE quirkle_items SCHEMAFULL;").await?;
    db.query("DEFINE FIELD quirkle_root ON quirkle_items TYPE string;")
        .await?;
    db.query("DEFINE FIELD address ON quirkle_items TYPE string;")
        .await?;

    db.query("DEFINE TABLE transaction_outputs SCHEMAFULL;")
        .await?;
    db.query("DEFINE FIELD transaction_hash ON transaction_outputs TYPE string;")
        .await?;
    db.query("DEFINE FIELD output_index ON transaction_outputs TYPE int;")
        .await?;
    db.query("DEFINE FIELD output ON transaction_outputs FLEXIBLE TYPE object;")
        .await?;
    db.query("DEFINE FIELD spent ON transaction_outputs TYPE bool;")
        .await?;

    db.query("DEFINE TABLE objects SCHEMAFULL;").await?;
    db.query("DEFINE FIELD object_id ON objects TYPE string;")
        .await?;
    db.query("DEFINE FIELD cert_ttl ON objects TYPE int;")
        .await?;
    db.query("DEFINE FIELD claims ON objects FLEXIBLE TYPE array;")
        .await?;

    Ok(())
}
