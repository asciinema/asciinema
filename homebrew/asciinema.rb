require 'formula'

class Asciinema < Formula
  homepage 'http://asciinema.org'
  url 'https://github.com/sickill/asciinema/archive/v0.9.4.tar.gz'
  sha1 'bf9292a133ca6bf37238110762bee60e99882c85'

  depends_on :python => "2.7"

  def install
    system 'make', 'install', "PREFIX=#{prefix}"
  end
end
