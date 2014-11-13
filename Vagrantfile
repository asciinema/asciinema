# -*- mode: ruby -*-
# vi: set ft=ruby :

# Vagrantfile API/syntax version. Don't touch unless you know what you're doing!
VAGRANTFILE_API_VERSION = "2"

Vagrant.configure(VAGRANTFILE_API_VERSION) do |config|

  config.vm.provider :virtualbox do |vb|
    vb.customize ["modifyvm", :id, "--memory", "1024"]
  end

  config.vm.define "archlinux" do |c|
    c.vm.box = "terrywang/archlinux"
  end

  config.vm.define "ubuntu" do |c|
    c.vm.box = "ubuntu/trusty64"
  end

end
