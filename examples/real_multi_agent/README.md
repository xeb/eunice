# Restaurant Multi-Agent Simulation

This example demonstrates eunice's multi-agent capabilities by simulating a restaurant with multiple cooperating agents.

## Agents

| Agent | Role | Model | Can Invoke |
|-------|------|-------|------------|
| `root` (counter) | Takes customer orders | default | head_chef |
| `head_chef` | Coordinates kitchen | default | line_cook, supplier |
| `line_cook` | Prepares dishes | gemini-3-flash-preview | - |
| `supplier` | Manages inventory | gemini-3-flash-preview | - |

Agents can specify their own model. Simpler agents (line_cook, supplier) use the faster flash model, while complex coordination agents use the default model.

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

# See agent configurations and models
eunice --list-agents
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

## Tool Access Control

Each agent has specific tool access using pattern matching:

```bash
# See what tools each agent can access
eunice --list-tools
```

| Agent | Tools | Purpose |
|-------|-------|---------|
| counter (root) | `filesystem_read_file`, `filesystem_list_directory` | Read-only (menus, orders) |
| head_chef | `filesystem_read_file`, `filesystem_write_file`, `filesystem_edit_file` | Coordinate orders |
| line_cook | `filesystem_*`, `shell_*` | Full access for cooking |
| supplier | `filesystem_read_file`, `filesystem_write_file`, `filesystem_edit_file` | Manage inventory |

Tool patterns support wildcards: `filesystem_*` matches all filesystem tools.

## Customization

Edit the agent prompts in `agents/` to change behavior:
- `agents/counter.md` - Customer interaction style
- `agents/head_chef.md` - Kitchen coordination
- `agents/line_cook.md` - Cooking procedures
- `agents/supplier.md` - Inventory management
