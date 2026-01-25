# Docker Installation Controls
# Verifies that Docker is properly installed and configured

control 'docker-01' do
  impact 1.0
  title 'Docker is installed'
  desc 'Verify that Docker is installed and running'

  describe package('docker-ce') do
    it { should be_installed }
  end

  describe service('docker') do
    it { should be_enabled }
    it { should be_running }
  end

  describe command('docker --version') do
    its('exit_status') { should eq 0 }
    its('stdout') { should match(/Docker version/) }
  end
end

control 'docker-02' do
  impact 0.7
  title 'Docker daemon is accessible'
  desc 'Verify that Docker daemon is accessible'

  describe command('docker info') do
    its('exit_status') { should eq 0 }
  end
end

control 'docker-03' do
  impact 0.5
  title 'Ubuntu user is in docker group'
  desc 'Verify that ubuntu user can run docker commands without sudo'

  only_if { user('ubuntu').exists? }

  describe user('ubuntu') do
    its('groups') { should include 'docker' }
  end
end
