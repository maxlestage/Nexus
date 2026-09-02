#!/usr/bin/env ruby
# frozen_string_literal: true

# Déduit le Team ID Apple à partir de la seule clé App Store Connect.
#
# Évite d'exiger un secret de plus : l'attribut `seedId` d'un identifiant de
# bundle EST le Team ID. On privilégie l'identifiant de l'application, et à
# défaut on prend le premier du compte.
#
# Variables attendues : ASC_KEY_ID, ASC_ISSUER_ID, ASC_KEY_PATH, BUNDLE_ID.
# Écrit le Team ID sur la sortie standard.

require "jwt"
require "json"
require "net/http"
require "openssl"
require "uri"

def fail_with(message)
  warn "team-id: #{message}"
  exit 1
end

key_id = ENV["ASC_KEY_ID"] or fail_with("ASC_KEY_ID manquant")
issuer_id = ENV["ASC_ISSUER_ID"] or fail_with("ASC_ISSUER_ID manquant")
key_path = ENV["ASC_KEY_PATH"] or fail_with("ASC_KEY_PATH manquant")
bundle_id = ENV["BUNDLE_ID"].to_s

fail_with("clé introuvable : #{key_path}") unless File.exist?(key_path)

now = Time.now.to_i
token = JWT.encode(
  { iss: issuer_id, iat: now, exp: now + 600, aud: "appstoreconnect-v1" },
  OpenSSL::PKey::EC.new(File.read(key_path)),
  "ES256",
  { kid: key_id, typ: "JWT" }
)

uri = URI("https://api.appstoreconnect.apple.com/v1/bundleIds?limit=200")
request = Net::HTTP::Get.new(uri)
request["Authorization"] = "Bearer #{token}"

response = Net::HTTP.start(uri.hostname, uri.port, use_ssl: true, read_timeout: 30) do |http|
  http.request(request)
end

unless response.is_a?(Net::HTTPSuccess)
  fail_with("App Store Connect a répondu #{response.code} : #{response.body.to_s[0, 300]}")
end

entries = JSON.parse(response.body).fetch("data", [])
fail_with("aucun identifiant de bundle dans ce compte") if entries.empty?

exact = entries.find { |e| e.dig("attributes", "identifier") == bundle_id }
seed = (exact || entries.first).dig("attributes", "seedId")
fail_with("seedId absent de la réponse") if seed.to_s.empty?

warn(exact ? "team-id: déduit de #{bundle_id}" : "team-id: déduit du premier identifiant du compte")
puts seed
