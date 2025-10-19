{
  age.sshKeyPaths = [ "/etc/ssh/ssh_host_ed25519_key" ];
  defaultSopsFile = ./secrets.yaml;
  secrets = {
    aemet_key = {};
    aemet_station = {};
    openweatherkey = {};
    tide_station_id = {};
  };
}
