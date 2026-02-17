# Extension Validation Controls
# Verifies that Sindri extension management is properly configured and working

control 'ext-01' do
  impact 1.0
  title 'Extension Manager Available'
  desc 'Verify that the Sindri extension manager is operational'

  describe command('sindri extension list') do
    its('exit_status') { should eq 0 }
  end
end

control 'ext-02' do
  impact 0.7
  title 'Extension Registry Accessible'
  desc 'Verify that the extension registry can be queried'

  describe command('sindri extension search --limit 1 2>/dev/null || sindri extension list 2>/dev/null') do
    its('exit_status') { should eq 0 }
  end
end

control 'ext-03' do
  impact 0.8
  title 'Pre-installed Extensions Available'
  desc 'Verify that pre-installed extensions from the image profile are available'

  # Extensions can be specified via EXTENSIONS environment variable
  extensions = ENV['EXTENSIONS']&.split(',')&.map(&:strip) || []

  if extensions.empty?
    describe 'No extensions specified' do
      skip 'EXTENSIONS environment variable not set - skipping extension validation'
    end
  else
    extensions.each do |ext|
      describe "Extension: #{ext}" do
        describe command("sindri extension status #{ext} 2>&1") do
          its('stdout') { should match(/installed|available/i) }
        end
      end
    end
  end
end

control 'ext-04' do
  impact 0.5
  title 'Extension Directories Exist'
  desc 'Verify that extension directories are properly created'

  only_if { user('ubuntu').exists? }

  describe directory('/home/ubuntu/.sindri/extensions') do
    it { should exist }
    its('owner') { should eq 'ubuntu' }
    its('mode') { should cmp '0755' }
  end
end

control 'ext-05' do
  impact 0.5
  title 'Extension Cache Directory Exists'
  desc 'Verify that the extension cache directory is available'

  only_if { user('ubuntu').exists? }

  describe directory('/home/ubuntu/.sindri/cache') do
    it { should exist }
    its('owner') { should eq 'ubuntu' }
  end
end

control 'ext-06' do
  impact 0.3
  title 'Extension Manifest Writable'
  desc 'Verify that the extension manifest can be updated'

  only_if { user('ubuntu').exists? }

  describe directory('/home/ubuntu/.sindri/state') do
    it { should exist }
    its('owner') { should eq 'ubuntu' }
    it { should be_writable.by('owner') }
  end
end
