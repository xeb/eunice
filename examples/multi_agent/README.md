# Multi-Agent Story Writing Example

This example demonstrates a sophisticated multi-agent workflow where eunice orchestrates different "agents" to collaboratively create and refine cyberpunk stories.

## How It Works

The example implements a three-agent system:

1. **Writer Agent** (`agent_writer.txt`) - Creates initial cyberpunk stories about data couriers with memory implants
2. **Editor Agent** (`agent_editor.txt`) - Improves story pacing, character development, and narrative flow
3. **Publisher Agent** (`agent_publisher.txt`) - Evaluates stories and decides if they meet publication standards

## Workflow

1. Read writer agent role and create initial story → `story.txt`
2. Read editor agent role and improve the story → `story_edited.txt`
3. Read publisher agent role and evaluate the story → `story_publisher_result.txt`
4. If rejected, repeat up to 3 times with improvements based on feedback

## Usage

```bash
./run.sh
```

## Configuration

Uses `eunice_minimal.json` with:
- **filesystem** - For reading/writing story files
- **memory** - For tracking iterations and feedback

## Generated Files

- `story.txt` - Initial story from writer agent
- `story_edited.txt` - Improved version from editor agent
- `story_publisher_result.txt` - Publisher evaluation (TRUE/FALSE/REJECTED)

## Key Features Demonstrated

- Complex multi-step agentic workflows
- File I/O operations via MCP filesystem server
- Memory persistence for tracking state between iterations
- Iterative improvement based on structured feedback
- Automatic retry logic for quality assurance