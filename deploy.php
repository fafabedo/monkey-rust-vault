<?php

namespace Deployer;

require 'recipe/common.php';

set('application', 'monkey-vault');
set('keep_releases', 3);

set('repository', 'git@github.com:bedoyatech/monkey-rust-vault.git');

set('shared_files', ['.env']);
set('shared_dirs', []);
set('writable_dirs', []);

// ─── Hosts ───────────────────────────────────────────────────────────────────

host('vault.monkeylibrary.app')
  ->stage('prod')
  ->user('fabricio')
  ->identityFile('~/.ssh/id_ed25519')
  ->forwardAgent(true)
  ->set('deploy_path', '/home/fabricio/Apps/monkey-vault');

host('vault.venux-channel.com')
  ->stage('staging')
  ->user('fabricio')
  ->identityFile('~/.ssh/id_rsa')
  ->forwardAgent(true)
  ->set('deploy_path', '/home/fabricio/Apps/monkey-vault');

// ─── Tasks ───────────────────────────────────────────────────────────────────

task('app:build', function () {
  cd('{{release_path}}');
  run('sh install.sh', ['timeout' => null]);
});

task('app:restart', function () {
  run('sudo systemctl restart monkey-vault');
});

task('deploy', [
  'deploy:info',
  'deploy:prepare',
  'deploy:lock',
  'deploy:release',
  'deploy:update_code',
  'deploy:shared',
  'deploy:writable',
  'app:build',
  'deploy:clear_paths',
  'deploy:symlink',
  'app:restart',
  'deploy:unlock',
  'cleanup',
]);

after('deploy:failed', 'deploy:unlock');
