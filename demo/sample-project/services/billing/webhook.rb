require "json"
require "openssl"

class Webhook
  def initialize(secret)
    @secret = secret
  end

  def verify(payload, signature)
    expected = OpenSSL::HMAC.hexdigest("SHA256", @secret, payload)
    Rack::Utils.secure_compare(expected, signature)
  end

  def handle(payload)
    event = JSON.parse(payload)
    puts "invoice #{event["id"]} paid by #{event["customer"]}"
  end
end
