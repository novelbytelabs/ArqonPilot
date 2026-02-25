use anyhow::{Error, Result};
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use tokenizers::Tokenizer;

pub struct MiniLM {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl MiniLM {
    pub fn new() -> Result<Self> {
        let device = Device::Cpu; // Force CPU for simplicity/compatibility

        // Load model and tokenizer from HuggingFace Hub (or local cache)
        let model_id = "sentence-transformers/all-MiniLM-L6-v2".to_string();
        let _repo = candle_core::utils::cuda_is_available(); // Only used to trigger download really

        let api = hf_hub::api::sync::Api::new()?;
        let repo = api.model(model_id);

        let config_filename = repo.get("config.json")?;
        let tokenizer_filename = repo.get("tokenizer.json")?;
        let weights_filename = repo.get("model.safetensors")?;

        let config = std::fs::read_to_string(config_filename)?;
        let config: Config = serde_json::from_str(&config)?;

        let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(Error::msg)?;
        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_filename], DTYPE, &device)? };

        let model = BertModel::load(vb, &config)?;

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>> {
        let tokenizer = self
            .tokenizer
            .with_padding(None)
            .with_truncation(None)
            .map_err(Error::msg)?;
        let tokens = tokenizer.encode(text, true).map_err(Error::msg)?;

        let token_ids = Tensor::new(tokens.get_ids(), &self.device)?.unsqueeze(0)?;
        let token_type_ids = Tensor::new(tokens.get_type_ids(), &self.device)?.unsqueeze(0)?;

        let embeddings = self.model.forward(&token_ids, &token_type_ids, None)?;

        // Mean pooling
        let (_n_sentence, n_tokens, _hidden_size) = embeddings.dims3()?;
        let embeddings = (embeddings.sum(1)? / (n_tokens as f64))?;
        let embeddings = normalize(&embeddings)?;

        let vec = embeddings.squeeze(0)?.to_vec1::<f32>()?;
        Ok(vec)
    }
}

fn normalize(tensor: &Tensor) -> Result<Tensor> {
    let sum_sq = tensor.sqr()?.sum_all()?.sqrt()?;
    Ok(tensor.broadcast_div(&sum_sq)?)
}
