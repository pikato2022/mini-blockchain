use chrono::Utc;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
pub fn hash_to_binary_representation(hash: &[u8]) -> String {
    let mut res: String = String::default();
    for c in hash {
        res.push_str(&format!("{:b}", c));
    }
    res
}

pub fn mine_block(id: u64, timestamp: i64, previous_hash: &str, data: &str) -> (u64, String) {
    info!("mining block");
    let mut nonce = 0;
    loop {
        if nonce % 100000 == 0 {
            info!("nonce {}", nonce);
        }
        let hash = calculate_hash(id, timestamp, previous_hash, data, nonce);
        let binary_hash = hash_to_binary_representation(&hash);
        if binary_hash.starts_with(DIFFICULTY_PREFIX) {
            info!(
                "mined nonce {}, hash {}, binary_hash {}",
                nonce,
                hex::encode(&hash),
                binary_hash
            );
            return (nonce, hex::encode(&hash));
        }
        nonce += 1;
    }
}
pub fn calculate_hash(
    id: u64,
    timestamp: i64,
    previous_hash: &str,
    data: &str,
    nonce: u64,
) -> Vec<u8> {
    let data = serde_json::json!({
        "id": id,
        "previous_hash": previous_hash,
        "data": data,
        "timestamp": timestamp,
        "nonce": nonce
    });
    let mut hasher = Sha256::new();
    hasher.update(data.to_string().as_bytes());
    hasher.finalize().as_slice().to_owned()
}
pub struct App {
    pub blocks: Vec<Block>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub id: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: i64,
    pub data: String,
    pub nonce: u64,
}
impl Block {
    pub fn calculate_hash(&self) -> Vec<u8> {
        calculate_hash(
            self.id,
            self.timestamp,
            &self.previous_hash,
            &self.data,
            self.nonce,
        )
    }
    pub fn new(id: u64, previous_hash: String, data: String) -> Self {
        let now = Utc::now();
        let (nonce, hash) = mine_block(id, now.timestamp(), &previous_hash, &data);
        Self {
            id,
            hash,
            previous_hash,
            timestamp: now.timestamp(),
            data,

            nonce,
        }
    }
}
const DIFFICULTY_PREFIX: &str = "00";

impl App {
    pub fn new() -> Self {
        Self { blocks: vec![] }
    }
    pub fn genesis(&mut self) {
        let genesis_block = Block {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("genesis"),
            data: String::from("genesis"),
            nonce: 2836,
            hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),
        };
        self.blocks.push(genesis_block);
    }
    pub fn try_add_block(&mut self, block: Block) {
        let latest_block = self.blocks.last().expect("there is at least one block");
        if self.is_block_valid(&block, latest_block) {
            self.blocks.push(block);
        } else {
            error!("could not add block - invalid");
        }
    }
    pub fn mine_new_block(&mut self, id: u64, previous_hash: String, data: String) {
        let block = Block::new(id, previous_hash, data);
        self.try_add_block(block);
    }
    pub fn is_block_valid(&self, block: &Block, previous_block: &Block) -> bool {
        //check valid previous hash in next block
        if block.previous_hash != previous_block.hash {
            warn!(
                "block with id {} has wrong previous hash {} and right previous hash {}",
                block.id, block.previous_hash, previous_block.hash
            );
            return false;
        }
        //check valid hash by prefix
        if !hash_to_binary_representation(&hex::decode(&block.hash).expect("can decode from hex"))
            .starts_with(DIFFICULTY_PREFIX)
        {
            warn!("block with id {} has invalid difficulty", block.id);
            return false;
        }
        //check block is next block by id
        if block.id != previous_block.id + 1 {
            warn!("block with id {} is not continuous", block.id);
            return false;
        }
        //check valid hash by hash data
        if hex::encode(block.calculate_hash()) != block.hash {
            warn!("block with id {} has invalid hash", block.id);
            return false;
        }
        true
    }

    pub fn is_chain_valid(&self, chain: &[Block]) -> bool {
        for i in 0..chain.len() {
            if i == 0 {
                continue;
            }
            let first = chain.get(i - 1).expect("has to exist");
            let second = chain.get(i).expect("has to exist");
            if !self.is_block_valid(second, first) {
                return false;
            }
        }
        true
    }
    // always choose the longest valid chain
    pub fn choose_chain(&mut self, local: Vec<Block>, remote: Vec<Block>) -> Vec<Block> {
        let is_local_valid = self.is_chain_valid(&local);
        let is_remote_valid = self.is_chain_valid(&remote);
        if is_local_valid && is_remote_valid {
            if local.len() >= remote.len() {
                local
            } else {
                remote
            }
        } else if !is_local_valid && is_remote_valid {
            remote
        } else if !is_remote_valid && is_local_valid {
            local
        } else {
            panic!("local and remote chain are both invalid");
        }
    }
}
