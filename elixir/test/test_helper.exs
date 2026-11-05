# A unit test runs no external command: each answers through the handler the test
# installs in Dotfiles.FakeCommand, and one the test did not install fails it. An
# integration test puts back the real implementation for itself.
Application.put_env(:dotfiles, :command, Dotfiles.FakeCommand)

ExUnit.start()
