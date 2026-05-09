import re

with open('src/main.rs', 'r') as f:
    content = f.read()

# Inspect what's actually in main.rs because the previous python script failed silently to replace things

print("1. main_window_size in TradingTerminal struct:")
print(re.search(r'main_window_size', content))

print("\n2. WalletTrackerState struct:")
print(re.search(r'pub struct WalletTrackerState \{[^}]*\}', content, re.DOTALL).group(0))

print("\n3. WindowMoved in match block:")
print(re.search(r'Message::WindowMoved', content[2500:5500])) # The match block is huge, but let's see if it's there
