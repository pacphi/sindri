# Mise Environment Controls
# Verifies that mise (tool version manager) is properly installed and configured

control 'mise-01' do
  impact 1.0
  title 'Mise is Installed'
  desc 'Verify that mise CLI is installed and accessible'

  describe command('mise --version') do
    its('exit_status') { should eq 0 }
    its('stdout') { should match(/mise \d+\.\d+/) }
  end
end

control 'mise-02' do
  impact 0.7
  title 'Mise Shims Directory Exists'
  desc 'Verify that mise shims directory is in PATH'

  only_if { user('ubuntu').exists? }

  describe directory('/home/ubuntu/.local/share/mise/shims') do
    it { should exist }
  end

  # Check if shims are in PATH
  describe command('echo $PATH') do
    its('stdout') { should match(%r{\.local/share/mise/shims}) }
  end
end

control 'mise-03' do
  impact 0.5
  title 'Mise Configuration Directory Exists'
  desc 'Verify that mise configuration directory structure exists'

  only_if { user('ubuntu').exists? }

  describe directory('/home/ubuntu/.config/mise') do
    it { should exist }
    its('owner') { should eq 'ubuntu' }
  end

  describe directory('/home/ubuntu/.config/mise/conf.d') do
    it { should exist }
    its('owner') { should eq 'ubuntu' }
  end
end

control 'mise-04' do
  impact 0.5
  title 'Mise is Activated'
  desc 'Verify that mise is properly activated in the shell'

  # Check for mise activation in shell profile
  describe.one do
    describe file('/home/ubuntu/.bashrc') do
      its('content') { should match(/mise activate/) }
    end
    describe file('/home/ubuntu/.zshrc') do
      its('content') { should match(/mise activate/) }
    end
    describe file('/home/ubuntu/.profile') do
      its('content') { should match(/mise activate/) }
    end
  end
end

control 'mise-05' do
  impact 0.3
  title 'Mise Doctor Passes'
  desc 'Verify that mise doctor reports no critical issues'

  describe command('mise doctor 2>&1') do
    its('exit_status') { should eq 0 }
  end
end

control 'mise-06' do
  impact 0.5
  title 'Mise Tools Installed'
  desc 'Verify that expected mise tools are installed'

  # Tools can be specified via MISE_TOOLS environment variable
  tools = ENV['MISE_TOOLS']&.split(',')&.map(&:strip) || []

  if tools.empty?
    describe 'No tools specified' do
      skip 'MISE_TOOLS environment variable not set - skipping tool validation'
    end
  else
    tools.each do |tool|
      describe "Mise tool: #{tool}" do
        describe command("mise list #{tool.split('@').first} 2>&1") do
          its('exit_status') { should eq 0 }
          its('stdout') { should_not match(/No versions installed/) }
        end
      end
    end
  end
end
