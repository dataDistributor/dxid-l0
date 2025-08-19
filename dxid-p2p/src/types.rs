// in dxid-p2p/src/types.rs
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GossipTx {
  pub id: String,            // tx hash hex
  pub body: serde_json::Value, // original SubmitTxReq or minimal wire form
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GossipBlock {
  pub height: u64,
  pub hash: String,
  pub parent_hash: String,
  pub state_root: String,
  pub tx_ids: Vec<String>,
  pub body: serde_json::Value, // your block header + minimal included txs
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Hello {
  pub chain_id: u32,
  pub genesis_hash: String,
  pub node_id: String, // PeerId string
}
