#!/usr/bin/env ruby
# regen-pangea-goldens.rb — render every bundled pangea architecture
# to terraform.json + write as the lava-equivalence golden fixture.
#
# Run from the pangea-architectures repo root:
#   bundle install
#   bundle exec ruby /path/to/lava-equivalence/scripts/regen-pangea-goldens.rb
#
# Output: writes JSON into lava-equivalence/tests/goldens/<arch>.json.
# Subsequent `cargo test --release` in lava-equivalence diffs lava
# render output against these pangea-generated goldens.

require 'json'
require 'fileutils'

# Resolve lava-equivalence golden dir relative to this script.
SCRIPT_DIR = File.expand_path(File.dirname(__FILE__))
GOLDENS_DIR = File.join(File.dirname(SCRIPT_DIR), 'tests', 'goldens')

# Boot pangea — load architectures + the synth helper.
$LOAD_PATH.unshift File.expand_path('../pangea-core/lib', SCRIPT_DIR) rescue nil
require 'pangea/core'
require 'pangea/architectures'
require 'pangea/synth/terraform_synthesizer'

# Map lava architecture name → (Pangea module, default config).
MAPPINGS = {
  'aws-vpc-network' => [Pangea::Architectures::AwsVpcNetwork, {}],
  'cloudflare-dns-records' => [Pangea::Architectures::CloudflareDnsRecords, {
    zone_id: '11112222333344445555666677778888',
    records: [
      { name: '@',  type: 'CNAME', content: 'example.com.cdn.cloudflare.net', proxied: true },
      { name: 'www', type: 'CNAME', content: 'example.com',                   proxied: true },
      { name: 'api', type: 'CNAME', content: 'origin.example.com',            proxied: true },
      { name: '*.staging', type: 'CNAME', content: 'example.com',             proxied: false },
      { name: '_acme-challenge', type: 'TXT', content: 'placeholder-acme-token', proxied: false }
    ]
  }],
  'akeyless-secrets' => [Pangea::Architectures::AkeylessSecrets, {}],
  'cloudflare-r2-bucket' => [Pangea::Architectures::CloudflareR2Bucket, {
    account_id: 'abcd1234abcd1234abcd1234abcd1234',
    bucket_name: 'demo-bucket'
  }],
  'public-dns' => [Pangea::Architectures::PublicDns, { name: 'main', domain: 'example.com' }],
  'akeyless-platform' => [Pangea::Architectures::AkeylessPlatform, {}]
}

FileUtils.mkdir_p(GOLDENS_DIR)

written = 0
MAPPINGS.each do |lava_name, (mod, config)|
  begin
    synth = Pangea::Synth::TerraformSynthesizer.new
    mod.build(synth, config)
    body = JSON.pretty_generate(synth.synthesis)
    out = File.join(GOLDENS_DIR, "#{lava_name}.json")
    File.write(out, body)
    puts "  ✓ #{lava_name} → #{File.size(out)} bytes"
    written += 1
  rescue StandardError => e
    puts "  ✗ #{lava_name}: #{e.class}: #{e.message}"
  end
end

puts "── #{written}/#{MAPPINGS.size} goldens regenerated ──"
