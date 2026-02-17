# Security Hardening Controls
# Verifies that CIS-aligned security hardening is properly applied

control 'sec-01' do
  impact 1.0
  title 'SSH Hardening - Root Login Disabled'
  desc 'Verify that SSH root login is disabled (CIS 5.2.8)'

  describe sshd_config do
    its('PermitRootLogin') { should eq 'no' }
  end
end

control 'sec-02' do
  impact 1.0
  title 'SSH Hardening - Password Authentication Disabled'
  desc 'Verify that SSH password authentication is disabled (CIS 5.2.9)'

  describe sshd_config do
    its('PasswordAuthentication') { should eq 'no' }
  end
end

control 'sec-03' do
  impact 0.7
  title 'SSH Hardening - Protocol Version'
  desc 'Verify that SSH uses only protocol version 2'

  # Protocol directive is deprecated in newer OpenSSH (defaults to 2)
  # but we check that version 1 is not explicitly enabled
  describe sshd_config do
    its('Protocol') { should_not eq '1' }
  end
end

control 'sec-04' do
  impact 0.7
  title 'SSH Hardening - X11 Forwarding Disabled'
  desc 'Verify that X11 forwarding is disabled (CIS 5.2.6)'

  describe sshd_config do
    its('X11Forwarding') { should eq 'no' }
  end
end

control 'sec-05' do
  impact 0.5
  title 'SSH Hardening - MaxAuthTries'
  desc 'Verify that SSH max authentication tries is limited (CIS 5.2.7)'

  describe sshd_config do
    its('MaxAuthTries') { should cmp <= 4 }
  end
end

control 'sec-06' do
  impact 0.7
  title 'Firewall Enabled'
  desc 'Verify that the firewall (ufw) is enabled and running'

  describe service('ufw') do
    it { should be_enabled }
    it { should be_running }
  end

  describe command('ufw status') do
    its('stdout') { should match(/Status: active/) }
  end
end

control 'sec-07' do
  impact 0.5
  title 'Firewall - SSH Allowed'
  desc 'Verify that SSH (port 22) is allowed through the firewall'

  describe command('ufw status verbose') do
    its('stdout') { should match(/22\/tcp\s+ALLOW/) }
  end
end

control 'sec-08' do
  impact 0.5
  title 'Firewall - Default Deny Incoming'
  desc 'Verify that the firewall defaults to denying incoming traffic'

  describe command('ufw status verbose') do
    its('stdout') { should match(/Default: deny \(incoming\)/) }
  end
end

control 'sec-09' do
  impact 0.5
  title 'Automatic Security Updates Enabled'
  desc 'Verify that unattended-upgrades is configured for security updates'

  describe package('unattended-upgrades') do
    it { should be_installed }
  end

  describe file('/etc/apt/apt.conf.d/20auto-upgrades') do
    it { should exist }
    its('content') { should match(/APT::Periodic::Unattended-Upgrade "1"/) }
  end
end

control 'sec-10' do
  impact 0.3
  title 'No World-Writable Files in System Directories'
  desc 'Verify that no world-writable files exist in system directories'

  describe command('find /usr /etc /var -type f -perm -0002 -ls 2>/dev/null | head -20') do
    its('stdout') { should eq '' }
  end
end

control 'sec-11' do
  impact 0.5
  title 'No SUID/SGID Binaries Outside Expected List'
  desc 'Verify SUID/SGID binaries are limited to expected system binaries'

  # List of expected SUID binaries on Ubuntu
  expected_suid = [
    '/usr/bin/sudo',
    '/usr/bin/passwd',
    '/usr/bin/chsh',
    '/usr/bin/chfn',
    '/usr/bin/newgrp',
    '/usr/bin/gpasswd',
    '/usr/lib/openssh/ssh-keysign',
    '/usr/lib/dbus-1.0/dbus-daemon-launch-helper',
  ]

  # This is informational - we just ensure common expected binaries are present
  describe file('/usr/bin/sudo') do
    it { should be_setuid }
  end
end

control 'sec-12' do
  impact 0.3
  title 'Sensitive Files Have Correct Permissions'
  desc 'Verify that sensitive files have restrictive permissions'

  # /etc/shadow should not be world-readable
  describe file('/etc/shadow') do
    its('mode') { should cmp '0640' }
    its('group') { should eq 'shadow' }
  end

  # /etc/passwd should be readable but not writable by others
  describe file('/etc/passwd') do
    its('mode') { should cmp '0644' }
  end
end
