# 🚨 Alert System Setup Guide

Follow these steps to connect your bot to Discord and Telegram. This will ensure you receive real-time notifications for trade successes, low balances, and health issues.

---

## 🔵 Discord Setup (Webhooks)

Discord webhooks are the easiest way to get rich, embedded notifications.

1.  **Open Discord**: Go to the server where you want to receive alerts.
2.  **Channel Settings**: Click the ⚙️ gear icon next to the channel name (e.g., `#alerts`).
3.  **Integrations**: Click on **Integrations** in the left sidebar.
4.  **Create Webhook**: Click **View Webhooks** -> **New Webhook**.
5.  **Copy URL**: Name it "HFT Bot" and click **Copy Webhook URL**.
6.  **Configure `.env`**:
    ```bash
    DISCORD_WEBHOOK_URL="https://discord.com/api/webhooks/YOUR_ID/YOUR_TOKEN"
    ```

---

## 🔵 Telegram Setup (BotFather)

Telegram provides a more mobile-friendly experience and supports HTML formatting.

### 1. Create your Bot
1.  Search for **@BotFather** on Telegram.
2.  Type `/newbot` and follow the instructions.
3.  **Copy API Token**: You will get a token like `123456789:ABCDefghIJKL_mnop`.
4.  **Configure `.env`**:
    ```bash
    TELEGRAM_BOT_TOKEN="123456789:ABCDefghIJKL_mnop"
    ```

### 2. Get your Chat ID
1.  Search for **@userinfobot** on Telegram.
2.  Send any message to it.
3.  **Copy ID**: It will reply with your numerical ID (e.g., `987654321`).
4.  **Configure `.env`**:
    ```bash
    TELEGRAM_CHAT_ID="987654321"
    ```

---

## 🧪 Testing the Alerts

To verify everything is working, you can simply start the bot. It is configured to send a **Success** alert immediately upon "Engine Ignition".

```bash
# Ensure your .env is updated
cargo run --package engine
```

### What to look for:
- **Discord**: A clean embed with "Identity", "Jito Status", and "RPC Endpoint".
- **Telegram**: A bold header saying **HFT Engine Started** followed by the setup details.

---

## ✅ Summary of Features
- **Tiered Severity**: Critical (Red), Warning (Yellow), Success (Green).
- **Anti-Spam**: Prevents duplicate alerts within a 5-minute window.
- **Smart Checks**: Low SOL balance and Stalled WebSocket detection.
