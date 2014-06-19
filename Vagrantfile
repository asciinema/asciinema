# -*- mode: ruby -*-
# vi: set ft=ruby :

# Vagrantfile API/syntax version. Don't touch unless you know what you're doing!
VAGRANTFILE_API_VERSION = "2"

Vagrant.configure(VAGRANTFILE_API_VERSION) do |config|

  config.vm.provider :virtualbox do |vb|
    vb.customize ["modifyvm", :id, "--memory", "1024"]
  end

  config.vm.define "arch" do |c|
    c.vm.box = "cameronmalek/arch1403"
    c.vm.provision "shell", inline: "pacman -Sy && pacman -S --noconfirm make python2-pip fakeroot binutils && pip2 install nose"
  end

  config.vm.define "ubuntu-raring" do |c|
    c.vm.box = "raring64"
    c.vm.box_url = "http://bit.ly/vagrant-lxc-raring64-2013-09-28-"
    c.vm.provision "shell", inline: "apt-get install python-pip make -y && pip install nose"
  end

  config.vm.define "fedora-19" do |c|
    c.vm.box = "fedora-19"
    c.vm.box_url = "https://dl.dropboxusercontent.com/u/86066173/fedora-19.box"
    c.vm.provision "shell", inline: "yum install python-pip make -y && pip install nose"
  end

end
