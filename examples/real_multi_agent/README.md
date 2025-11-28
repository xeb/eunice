# Restaurant Multi-Agent Simulation

This example demonstrates eunice's multi-agent capabilities by simulating a restaurant with multiple cooperating agents.

## Agents

| Agent | Role | Can Invoke |
|-------|------|------------|
| `root` (counter) | Takes customer orders | head_chef |
| `head_chef` | Coordinates kitchen | line_cook, supplier |
| `line_cook` | Prepares dishes | - |
| `supplier` | Manages inventory | - |

## How It Works

```
Customer → Counter → Head Chef → Supplier (check inventory)
                        ↓
                   Line Cook (prepare food)
                        ↓
                   Head Chef → Counter → Customer
```

## Running the Example

```bash
cd examples/real_multi_agent

# Order some food
eunice "I'd like to order a burger and fries please"

# Check the generated files
cat orders.txt
cat kitchen_log.txt
cat pantry.txt
```

## Requirements

- `mcpz` installed (`cargo install mcpz`)
- Or modify `eunice.toml` to use npx-based MCP servers

## What Happens

1. The **counter** (root agent) greets you and takes your order
2. Counter invokes **head_chef** with the order
3. Head chef invokes **supplier** to check ingredient availability
4. Supplier reads/creates `pantry.txt` and reports status
5. Head chef invokes **line_cook** to prepare the food
6. Line cook updates `pantry.txt` and logs to `kitchen_log.txt`
7. Results bubble back up to the counter
8. Counter confirms your order is ready

## Files Created

- `orders.txt` - Customer order log
- `kitchen_log.txt` - Kitchen activity log
- `pantry.txt` - Ingredient inventory

## Customization

Edit the agent prompts in `agents/` to change behavior:
- `agents/counter.md` - Customer interaction style
- `agents/head_chef.md` - Kitchen coordination
- `agents/line_cook.md` - Cooking procedures
- `agents/supplier.md` - Inventory management
