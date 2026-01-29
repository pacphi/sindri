# Guacamole Extension

> Version: 2.0.0 | Category: desktop | Last Updated: 2026-01-26

## Overview

Apache Guacamole web-based remote desktop gateway. Provides browser-based access to SSH, RDP, and VNC connections.

## What It Provides

| Tool             | Type      | License    | Description            |
| ---------------- | --------- | ---------- | ---------------------- |
| guacd            | server    | Apache-2.0 | Guacamole daemon       |
| guacamole-client | framework | Apache-2.0 | Web application        |
| tomcat           | server    | Apache-2.0 | Java servlet container |

## Requirements

- **Disk Space**: 2000 MB (2 GB)
- **Memory**: 2048 MB
- **Install Time**: ~180 seconds
- **Dependencies**: None

### Network Domains

- apache.org
- downloads.apache.org
- archive.apache.org
- archive.ubuntu.com
- security.ubuntu.com

## Installation

```bash
extension-manager install guacamole
```

## Configuration

### Environment Variables

| Variable         | Value              | Description                |
| ---------------- | ------------------ | -------------------------- |
| `GUACAMOLE_HOME` | /etc/guacamole     | Guacamole config directory |
| `CATALINA_HOME`  | /usr/share/tomcat9 | Tomcat installation        |

### Templates

| Template                      | Destination                         | Description      |
| ----------------------------- | ----------------------------------- | ---------------- |
| guacamole.properties.template | /etc/guacamole/guacamole.properties | Main config      |
| user-mapping.xml.template     | /etc/guacamole/user-mapping.xml     | User auth config |

### Install Method

Uses a custom installation script with 1200 second timeout. Installs version 1.5.4.

### Upgrade Strategy

Manual - run upgrade.sh script.

## Key Features

- **Web-based** - Access from any browser
- **Multi-protocol** - SSH, RDP, VNC support
- **No client needed** - HTML5 based
- **Session recording** - Optional recording
- **Authentication** - Multiple auth methods

## Usage Examples

### Accessing Guacamole

```bash
# Default URL after installation
# http://localhost:8080/guacamole

# Default credentials (change immediately!)
# Username: admin
# Password: admin
```

### Connection Configuration

```xml
<!-- /etc/guacamole/user-mapping.xml -->
<user-mapping>
    <authorize username="admin" password="admin">
        <connection name="SSH Server">
            <protocol>ssh</protocol>
            <param name="hostname">192.168.1.100</param>
            <param name="port">22</param>
            <param name="username">user</param>
        </connection>
        <connection name="Windows RDP">
            <protocol>rdp</protocol>
            <param name="hostname">192.168.1.101</param>
            <param name="port">3389</param>
        </connection>
    </authorize>
</user-mapping>
```

### Service Management

```bash
# Start guacd daemon
sudo systemctl start guacd

# Start Tomcat
sudo systemctl start tomcat9

# Check status
sudo systemctl status guacd
sudo systemctl status tomcat9

# View logs
sudo journalctl -u guacd
sudo tail -f /var/log/tomcat9/catalina.out
```

### Configuration Files

```bash
# Main configuration
sudo vim /etc/guacamole/guacamole.properties

# User authentication
sudo vim /etc/guacamole/user-mapping.xml

# Guacd configuration
sudo vim /etc/guacamole/guacd.conf
```

## Validation

The extension validates the following commands:

- `guacd --version` - Must match pattern `guacd version \d+\.\d+\.\d+`
- `java -version` - Must match pattern `version`

## Removal

```bash
extension-manager remove guacamole
```

**Requires confirmation.** Runs uninstall.sh script.

## Related Extensions

- [xfce-ubuntu](XFCE-UBUNTU.md) - XFCE desktop for Guacamole RDP access
