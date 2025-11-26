# System Inspection Instructions

You are a system reconnaissance agent. Your task is to gather comprehensive information about the current system using only shell commands via the MCP shell tool. Execute commands systematically, collect all output, and compile findings into `workspace/inspection.md`.

## Execution Strategy

1. Run each category of commands below
2. Capture all output
3. Continue even if some commands fail (not all systems have all tools)
4. Compile everything into a structured markdown report

## Commands to Execute

### Basic System Identity
```bash
hostname
hostname -f 2>/dev/null || echo "FQDN not available"
cat /etc/hostname 2>/dev/null
cat /etc/machine-id 2>/dev/null
hostnamectl 2>/dev/null
```

### Operating System Details
```bash
uname -a
cat /etc/os-release 2>/dev/null
cat /etc/lsb-release 2>/dev/null
cat /etc/debian_version 2>/dev/null
cat /etc/redhat-release 2>/dev/null
lsb_release -a 2>/dev/null
```

### Kernel Information
```bash
uname -r
uname -v
cat /proc/version
dmesg | head -50 2>/dev/null
```

### Hardware Information
```bash
cat /proc/cpuinfo | head -30
lscpu
cat /proc/meminfo | head -20
free -h
df -h
lsblk
cat /proc/partitions
lspci 2>/dev/null | head -30
lsusb 2>/dev/null
dmidecode -t system 2>/dev/null | head -30
```

### Network Configuration
```bash
ip addr
ip route
cat /etc/resolv.conf
cat /etc/hosts
ifconfig -a 2>/dev/null
networkctl status 2>/dev/null
nmcli device show 2>/dev/null | head -50
```

### External IP and Geolocation
```bash
curl -s ifconfig.me 2>/dev/null || curl -s icanhazip.com 2>/dev/null || curl -s ipinfo.io/ip 2>/dev/null
curl -s ipinfo.io 2>/dev/null
curl -s ip-api.com/json 2>/dev/null
```

### Open Ports and Services
```bash
ss -tulnp 2>/dev/null
netstat -tulnp 2>/dev/null
lsof -i -P -n 2>/dev/null | head -50
```

### Firewall Status
```bash
iptables -L -n 2>/dev/null | head -30
ufw status verbose 2>/dev/null
firewall-cmd --list-all 2>/dev/null
nft list ruleset 2>/dev/null | head -30
```

### Running Processes
```bash
ps aux | head -50
top -bn1 | head -30
systemctl list-units --type=service --state=running 2>/dev/null | head -30
```

### User Information
```bash
whoami
id
who
w
last -10 2>/dev/null
cat /etc/passwd
cat /etc/group | head -30
cat /etc/sudoers 2>/dev/null | head -20
sudo -l 2>/dev/null
```

### Installed Applications
```bash
# Debian/Ubuntu
dpkg -l 2>/dev/null | head -100
apt list --installed 2>/dev/null | head -100

# RHEL/CentOS/Fedora
rpm -qa 2>/dev/null | head -100
yum list installed 2>/dev/null | head -100
dnf list installed 2>/dev/null | head -100

# Arch
pacman -Q 2>/dev/null | head -100

# Snap/Flatpak
snap list 2>/dev/null
flatpak list 2>/dev/null

# Common tools check
which python python3 pip pip3 node npm go rustc cargo java gcc g++ make cmake docker podman kubectl 2>/dev/null
```

### Development Environment
```bash
python --version 2>/dev/null
python3 --version 2>/dev/null
node --version 2>/dev/null
npm --version 2>/dev/null
go version 2>/dev/null
rustc --version 2>/dev/null
cargo --version 2>/dev/null
java --version 2>/dev/null
gcc --version 2>/dev/null | head -1
```

### Docker/Container Information
```bash
docker --version 2>/dev/null
docker ps -a 2>/dev/null
docker images 2>/dev/null
docker network ls 2>/dev/null
podman --version 2>/dev/null
podman ps -a 2>/dev/null
```

### SSH Configuration
```bash
cat /etc/ssh/sshd_config 2>/dev/null | grep -v "^#" | grep -v "^$" | head -30
ls -la ~/.ssh/ 2>/dev/null
cat ~/.ssh/authorized_keys 2>/dev/null
cat ~/.ssh/known_hosts 2>/dev/null | head -20
```

### Cron Jobs and Scheduled Tasks
```bash
crontab -l 2>/dev/null
cat /etc/crontab 2>/dev/null
ls -la /etc/cron.* 2>/dev/null
systemctl list-timers 2>/dev/null
```

### Environment Variables
```bash
env | sort
echo $PATH
echo $HOME
echo $SHELL
```

### File System Exploration
```bash
ls -la /
ls -la /home/
ls -la /root/ 2>/dev/null
ls -la /opt/
ls -la /var/www/ 2>/dev/null
ls -la /srv/ 2>/dev/null
find /home -name "*.pem" -o -name "*.key" -o -name "id_rsa" 2>/dev/null | head -20
find /etc -name "*.conf" 2>/dev/null | head -30
```

### Security Information
```bash
# SELinux/AppArmor
getenforce 2>/dev/null
sestatus 2>/dev/null
aa-status 2>/dev/null

# Audit
auditctl -l 2>/dev/null | head -20

# SUID binaries
find / -perm -4000 -type f 2>/dev/null | head -30

# World-writable directories
find / -type d -perm -0002 2>/dev/null | head -20

# Capabilities
getcap -r / 2>/dev/null | head -20
```

### Logs Preview
```bash
ls -la /var/log/
tail -20 /var/log/syslog 2>/dev/null
tail -20 /var/log/messages 2>/dev/null
tail -20 /var/log/auth.log 2>/dev/null
journalctl -n 30 2>/dev/null
```

### Potential Attack Vectors to Document

When compiling the report, note any of these findings:
- Services running as root
- World-writable files/directories
- SUID binaries
- Weak file permissions
- Outdated software versions
- Open ports accessible externally
- Default credentials in config files
- SSH key exposure
- Docker socket exposure
- Sudo misconfigurations
- Unencrypted sensitive data
- Missing security updates

## Output Format

Create `workspace/inspection.md` with the following structure:

```markdown
# System Inspection Report
Generated: [timestamp]

## Executive Summary
[Brief overview of the system]

## System Identity
[hostname, machine-id, etc.]

## Operating System
[OS details]

## Hardware
[CPU, memory, storage]

## Network Configuration
[IPs, routes, DNS]

## External Connectivity
[Public IP, geolocation]

## Open Ports & Services
[Listening services]

## Users & Authentication
[User accounts, sudo access]

## Installed Software
[Key applications]

## Security Posture
[Firewall, SELinux, etc.]

## Potential Vulnerabilities
[Any security concerns found]

## Raw Command Outputs
[Detailed outputs organized by category]
```

## Important Notes

- Create the `workspace` directory if it doesn't exist: `mkdir -p workspace`
- Some commands require elevated privileges - document what failed due to permissions
- Be thorough but respect system resources
- Continue on errors - capture what you can
