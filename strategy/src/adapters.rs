// ONNX Model Adapter - Infrastructure layer implementation of AIModelPort

use anyhow::Result;
use mev_core::ArbitrageOpportunity;
use ort::{session::Session, value::Value, inputs};
use ndarray::Array1;
use crate::ports::AIModelPort;

/// ONNX-based AI model adapter
pub struct ONNXModelAdapter {
    session: Session,
}

impl ONNXModelAdapter {
    /// Create a new ONNX model adapter from a file path
    pub fn from_file(path: &str) -> Result<Self> {
        let session = Session::builder()?.commit_from_file(path)?;
        Ok(Self { session })
    }
}

impl AIModelPort for ONNXModelAdapter {
    fn predict_confidence(&self, opp: &ArbitrageOpportunity) -> Result<f32> {
        // Prepare input features matching train_model.py
        let route_liquidity = (opp.min_liquidity as f64).ln_1p() as f32;
        let profit_ratio = opp.expected_profit_lamports as f32 / opp.input_amount as f32;
        
        let input_data = Array1::from_vec(vec![
            opp.steps.len() as f32,          // num_hops
            opp.total_fees_bps as f32,
            opp.max_price_impact_bps as f32,
            route_liquidity,
            profit_ratio,
        ]);

        let input_tensor = input_data.insert_axis(ndarray::Axis(0));
        let input_value = Value::from_array(input_tensor.into_dyn())?;
        
        let outputs = self.session.run(inputs!["input" => input_value]?)?;
        
        // GradientBoostingClassifier output (probability)
        let output_tensor = outputs["variable"].try_extract_tensor::<f32>()?;
        
        Ok(output_tensor[[0, 0]])
    }
}

/// Mock AI model for testing - always returns high confidence
pub struct MockAIModel {
    confidence: f32,
}

impl MockAIModel {
    pub fn new(confidence: f32) -> Self {
        Self { confidence }
    }
}

impl AIModelPort for MockAIModel {
    fn predict_confidence(&self, _opp: &ArbitrageOpportunity) -> Result<f32> {
        Ok(self.confidence)
    }
}
