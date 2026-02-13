# Shell Inspection Example

This example demonstrates using Eunice's built-in Bash tool to perform comprehensive system reconnaissance. The agent executes shell commands to gather detailed information about the host system and compiles the findings into a structured markdown report.

## What It Does

The agent acts as a system reconnaissance tool that:

1. Executes a wide range of shell commands to gather system information
2. Collects data about hardware, network, users, services, and security configuration
3. Compiles all findings into `workspace/inspection.md`

Categories of information collected:
- System identity (hostname, machine-id)
- Operating system and kernel details
- Hardware (CPU, memory, storage)
- Network configuration and external IP
- Open ports and running services
- User accounts and authentication
- Installed software and development tools
- Security posture (firewall, SELinux/AppArmor)
- Potential vulnerabilities

## Usage

From this directory:

```bash
eunice --prompt instruction.md "Perform the system inspection"
```

Or create the run script:

```bash
./run.sh
```

Eunice will:
- Read the `instruction.md` file as the system prompt
- Use the built-in Bash tool to execute commands
- Create `workspace/inspection.md` with the compiled report

## Output

After running, you'll find a comprehensive system report at `workspace/inspection.md` containing:
- Executive summary
- Detailed system information organized by category
- Security observations and potential vulnerabilities
- Raw command outputs for reference

## Security Note

This example performs extensive system reconnaissance. Run only on systems you own or have explicit authorization to inspect. The commands are read-only and non-destructive, but they do reveal sensitive system information.
