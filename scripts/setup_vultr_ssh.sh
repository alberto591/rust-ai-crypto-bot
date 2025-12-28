#!/usr/bin/env bash
# Vultr SSH Setup Instructions
# Follow these steps to enable SSH key authentication

echo "🔐 Vultr SSH Setup Guide"
echo "========================"
echo ""

# Check if key exists
if [ ! -f ~/.ssh/vultr_mev_bot ]; then
    echo "📝 Step 1: Generate SSH Key"
    ssh-keygen -t ed25519 -C "vultr-mev-bot-$(date +%Y%m%d)" -f ~/.ssh/vultr_mev_bot -N ""
    echo "✅ Key generated!"
else
    echo "✅ SSH key already exists"
fi

echo ""
echo "📋 Step 2: Copy your PUBLIC key (everything below the line):"
echo "================================================================"
cat ~/.ssh/vultr_mev_bot.pub
echo "================================================================"
echo ""
echo "🌐 Step 3: Add key to Vultr via Dashboard:"
echo "  1. Go to: https://my.vultr.com/servers/"
echo "  2. Click on your server (149.28.35.68)"
echo "  3. Go to 'Settings' → 'SSH Keys'"
echo "  4. Click 'Add SSH Key'"
echo "  5. Paste the public key above"
echo "  6. Save"
echo ""
echo "OR via command line (if you have root password):"
echo "  cat ~/.ssh/vultr_mev_bot.pub | ssh root@149.28.35.68 'mkdir -p ~/.ssh && cat >> ~/.ssh/authorized_keys'"
echo ""
echo "✅ Step 4: Test connection:"
echo "  ssh -i ~/.ssh/vultr_mev_bot root@149.28.35.68"
echo ""
echo "📝 Step 5: Add to SSH config for convenience:"
echo "  echo 'Host vultr-mev' >> ~/.ssh/config"
echo "  echo '  HostName 149.28.35.68' >> ~/.ssh/config"
echo "  echo '  User root' >> ~/.ssh/config"
echo "  echo '  IdentityFile ~/.ssh/vultr_mev_bot' >> ~/.ssh/config"
echo ""
echo "Then you can connect with: ssh vultr-mev"
