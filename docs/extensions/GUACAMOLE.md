# Guacamole

Apache Guacamole web-based remote desktop gateway (SSH/RDP/VNC).

## Overview

| Property         | Value     |
| ---------------- | --------- |
| **Category**     | utilities |
| **Version**      | 2.0.0     |
| **Installation** | script    |
| **Disk Space**   | 1000 MB   |
| **Dependencies** | None      |

## Description

Apache Guacamole web-based remote desktop gateway (SSH/RDP/VNC) - provides browser-based access to remote desktops and servers without client software.

## Installed Tools

| Tool               | Type      | Version | Description            |
| ------------------ | --------- | ------- | ---------------------- |
| `guacd`            | server    | 1.5.4   | Guacamole proxy daemon |
| `guacamole-client` | framework | 1.5.4   | Web application        |
| `tomcat9`          | server    | dynamic | Java servlet container |

## Configuration

### Environment Variables

| Variable         | Value                | Scope  |
| ---------------- | -------------------- | ------ |
| `GUACAMOLE_HOME` | `/etc/guacamole`     | bashrc |
| `CATALINA_HOME`  | `/usr/share/tomcat9` | bashrc |

### Templates

| Template                        | Destination                           | Description         |
| ------------------------------- | ------------------------------------- | ------------------- |
| `guacamole.properties.template` | `/etc/guacamole/guacamole.properties` | Server config       |
| `user-mapping.xml.template`     | `/etc/guacamole/user-mapping.xml`     | User authentication |

### Sample User Mapping

```xml
<user-mapping>
  <authorize username="admin" password="password">
    <connection name="SSH Server">
      <protocol>ssh</protocol>
      <param name="hostname">localhost</param>
      <param name="port">22</param>
    </connection>
  </authorize>
</user-mapping>
```

## Network Requirements

- `downloads.apache.org` - Apache downloads
- `archive.apache.org` - Apache archive
- `archive.ubuntu.com` - Ubuntu packages
- `security.ubuntu.com` - Security updates

## Installation

```bash
extension-manager install guacamole
```

Installation uses specific version argument:

```yaml
script:
  args: ["--version", "1.5.4"]
```

## Access

After installation, access Guacamole at:

```text
http://localhost:8080/guacamole
```

Default credentials are configured in `user-mapping.xml`.

## Validation

```bash
guacd --version    # Expected: guacd version X.X.X
java -version
```

## Upgrade

**Strategy:** manual

```bash
extension-manager upgrade guacamole
```

## Removal

### Requires confirmation

```bash
extension-manager remove guacamole
```

## Related Extensions

- [xfce-ubuntu](XFCE-UBUNTU.md) - Desktop environment
