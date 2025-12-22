import pandas as pd
import numpy as np
import os
from sklearn.ensemble import GradientBoostingClassifier
from sklearn.preprocessing import StandardScaler
from sklearn.pipeline import Pipeline
from sklearn.model_selection import train_test_split, cross_val_score
from skl2onnx import convert_sklearn
from skl2onnx.common.data_types import FloatTensorType
import onnx

# Configuration
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT_DIR = os.path.dirname(SCRIPT_DIR)
ARBITRAGE_DATA_PATH = os.path.join(ROOT_DIR, "data", "arbitrage_data.csv")
MODEL_PATH = os.path.join(ROOT_DIR, "ai_model.onnx")

print("=" * 60)
print("Solana MEV Bot - AI Model Training (5-Hop Engine)")
print("=" * 60)

# 1. Load Arbitrage Data
if not os.path.exists(ARBITRAGE_DATA_PATH):
    print(f"Error: {ARBITRAGE_DATA_PATH} not found.")
    print("Please run the bot in DRY_RUN mode first to collect data:")
    print("  bash scripts/collect_live_data.sh")
    print("\nCreating synthetic data for demonstration...")
    
    # Synthetic data for testing
    data = pd.DataFrame({
        'timestamp': np.arange(1000),
        'num_hops': np.random.choice([3, 4, 5], 1000),
        'profit_lamports': np.random.randint(100_000, 50_000_000, 1000),
        'input_amount': np.full(1000, 1_000_000_000),
        'total_fees_bps': np.random.randint(30, 150, 1000),
        'max_price_impact_bps': np.random.randint(10, 100, 1000),
        'min_liquidity': np.random.randint(1_000_000_000, 100_000_000_000, 1000),
        'route': ['mock'] * 1000
    })
else:
    try:
        data = pd.read_csv(ARBITRAGE_DATA_PATH)
        print(f"✅ Loaded {len(data)} arbitrage opportunities from real data")
        print(f"   File: {ARBITRAGE_DATA_PATH}")
        print(f"   Columns: {list(data.columns)}")
    except Exception as e:
        print(f"Failed to read CSV: {e}")
        exit(1)

# 2. Feature Engineering
print("\n📊 Feature Engineering...")

# Profit ratio: expected profit relative to input
data['profit_ratio'] = data['profit_lamports'] / data['input_amount']

# Route liquidity normalized (minimum liquidity across path)
data['route_liquidity'] = np.log1p(data['min_liquidity'])  # Log scale for better distribution

# Target: Binary classification (1 if profitable above threshold, 0 otherwise)
PROFIT_THRESHOLD = 500_000  # 0.0005 SOL minimum profit
data['is_profitable'] = (data['profit_lamports'] > PROFIT_THRESHOLD).astype(int)

print(f"   - Profitable opportunities: {data['is_profitable'].sum()} / {len(data)}")
print(f"   - Profitability rate: {data['is_profitable'].mean():.2%}")

# Select features for model (5-hop aware)
feature_columns = [
    'num_hops',
    'total_fees_bps',
    'max_price_impact_bps',
    'route_liquidity',
    'profit_ratio'
]

X = data[feature_columns].astype(np.float32)
y = data['is_profitable'].astype(np.float32)

# Handle any missing values
X = X.fillna(0)

print(f"\n🔢 Feature Statistics:")
print(X.describe())

# 3. Train/Test Split
X_train, X_test, y_train, y_test = train_test_split(
    X, y, test_size=0.2, random_state=42, stratify=y
)

# 4. Train Pipeline (Scaling + Gradient Boosting Classifier)
print("\n🧠 Training Gradient Boosting Classifier...")
pipeline = Pipeline([
    ('scaler', StandardScaler()),
    ('classifier', GradientBoostingClassifier(
        n_estimators=100,
        learning_rate=0.1,
        max_depth=5,
        random_state=42
    ))
])

pipeline.fit(X_train, y_train)

# 5. Evaluate Model
train_score = pipeline.score(X_train, y_train)
test_score = pipeline.score(X_test, y_test)

print(f"\n📈 Model Performance:")
print(f"   - Train Accuracy: {train_score:.4f}")
print(f"   - Test Accuracy: {test_score:.4f}")

# Cross-validation
cv_scores = cross_val_score(pipeline, X, y, cv=5, scoring='accuracy')
print(f"   - Cross-Val Accuracy: {cv_scores.mean():.4f} (+/- {cv_scores.std():.4f})")

# 6. Convert to ONNX
print("\n🔧 Converting to ONNX format...")
initial_type = [('input', FloatTensorType([None, len(feature_columns)]))]

try:
    onnx_model = convert_sklearn(
        pipeline,
        initial_types=initial_type,
        target_opset=12,
        options={id(pipeline): {'zipmap': False}}
    )
    
    # Save model
    with open(MODEL_PATH, "wb") as f:
        f.write(onnx_model.SerializeToString())
    
    print(f"✅ Model saved to: {MODEL_PATH}")
    print(f"   - Input features: {len(feature_columns)}")
    print(f"   - Output: Confidence score (0-1)")
    
except Exception as e:
    print(f"❌ ONNX conversion failed: {e}")
    print("   Falling back to basic export...")
    exit(1)

print("\n" + "=" * 60)
print("🎉 Training Complete!")
print("=" * 60)
print("\nNext steps:")
print("  1. Update Rust code to use 5 features (num_hops, total_fees_bps, etc.)")
print("  2. Test the model: cargo run --package engine")
print("  3. Monitor confidence scores in TUI dashboard")
