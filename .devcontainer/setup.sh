#!/usr/bin/bash

echo PS1="\"\[\033[1;35m\]\u\[\033[0m\]@\[\033[1;36m\]\h\[\033[0m\] \[\033[34m\]\w\[\033[0m\]\n└> \[\033[1;32m\]$\[\033[0m\] \"" >> ~/.bashrc
echo alias ls="\"eza --icons -lah --sort=Name --group-directories-first --git\"" >> ~/.bashrc
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Sway, for testing. Run with WLR_BACKENDS=headless sway
# Adding runtime dir as it do not exist.
echo export XDG_RUNTIME_DIR="/run/user/dev" >> ~/.bashrc
sudo mkdir /run/user/dev
sudo chown dev /run/user/dev
chmod 0700 /run/user/dev
# As sway created the socket named wayland-1 instead of wayland-0, we going to adjust the WAYLAND_DISPLAY envar
echo export WAYLAND_DISPLAY=wayland-1 >> ~/.bashrc

# for some reason CodeLLDB is hard coded to /usr/bin/cargo, exposing cargo in $PATH or setting lldb.cargo is not working.
sudo ln -s /home/dev/.cargo/bin/cargo /usr/bin/cargo