# Sindri Installation Controls
# Verifies that Sindri CLI is properly installed and configured

control 'sindri-01' do
  impact 1.0
  title 'Sindri CLI is installed'
  desc 'Verify that the Sindri CLI binary is installed and executable'

  describe file('/usr/local/bin/sindri') do
    it { should exist }
    it { should be_file }
    it { should be_executable }
  end

  describe command('sindri --version') do
    its('exit_status') { should eq 0 }
    its('stdout') { should match(/sindri \d+\.\d+\.\d+/) }
  end
end

control 'sindri-02' do
  impact 0.7
  title 'Sindri directories exist'
  desc 'Verify that Sindri configuration directories are created'

  only_if { user('ubuntu').exists? }

  describe directory('/home/ubuntu/.sindri') do
    it { should exist }
    its('owner') { should eq 'ubuntu' }
  end

  describe directory('/home/ubuntu/.sindri/state') do
    it { should exist }
    its('owner') { should eq 'ubuntu' }
  end

  describe directory('/home/ubuntu/.sindri/extensions') do
    it { should exist }
    its('owner') { should eq 'ubuntu' }
  end

  describe directory('/home/ubuntu/.sindri/cache') do
    it { should exist }
    its('owner') { should eq 'ubuntu' }
  end
end

control 'sindri-03' do
  impact 0.5
  title 'Sindri environment is configured'
  desc 'Verify that Sindri environment variables are set'

  describe file('/etc/profile.d/sindri.sh') do
    it { should exist }
    its('content') { should match(/SINDRI_HOME/) }
    its('content') { should match(/SINDRI_EXTENSIONS_DIR/) }
  end
end

control 'sindri-04' do
  impact 0.7
  title 'Sindri doctor passes'
  desc 'Verify that sindri doctor reports no critical issues'

  describe command('sindri doctor 2>&1') do
    its('exit_status') { should eq 0 }
  end
end
