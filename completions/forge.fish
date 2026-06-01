# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_forge_global_optspecs
	string join \n v/verbose q/quiet h/help V/version
end

function __fish_forge_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_forge_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_forge_using_subcommand
	set -l cmd (__fish_forge_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c forge -n "__fish_forge_needs_command" -s v -l verbose -d 'Enable verbose logging'
complete -c forge -n "__fish_forge_needs_command" -s q -l quiet -d 'Suppress all output except errors'
complete -c forge -n "__fish_forge_needs_command" -s h -l help -d 'Print help'
complete -c forge -n "__fish_forge_needs_command" -s V -l version -d 'Print version'
complete -c forge -n "__fish_forge_needs_command" -f -a "init" -d 'Initialize a new Reticulum network project'
complete -c forge -n "__fish_forge_needs_command" -f -a "generate" -d 'Generate interface configs for hardware'
complete -c forge -n "__fish_forge_needs_command" -f -a "simulate" -d 'Simulate a virtual Reticulum network'
complete -c forge -n "__fish_forge_needs_command" -f -a "deploy" -d 'Deploy configs to remote nodes'
complete -c forge -n "__fish_forge_needs_command" -f -a "test" -d 'Test network config, connectivity, and policies'
complete -c forge -n "__fish_forge_needs_command" -f -a "monitor" -d 'Real-time network health dashboard (TUI)'
complete -c forge -n "__fish_forge_needs_command" -f -a "completions" -d 'Generate shell completions for bash, zsh, fish, or PowerShell'
complete -c forge -n "__fish_forge_needs_command" -f -a "man" -d 'Generate man page for forge'
complete -c forge -n "__fish_forge_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c forge -n "__fish_forge_using_subcommand init" -s t -l topology -d 'Network topology template' -r
complete -c forge -n "__fish_forge_using_subcommand init" -s v -l verbose -d 'Enable verbose logging'
complete -c forge -n "__fish_forge_using_subcommand init" -s q -l quiet -d 'Suppress all output except errors'
complete -c forge -n "__fish_forge_using_subcommand init" -s h -l help -d 'Print help'
complete -c forge -n "__fish_forge_using_subcommand generate" -s H -l hardware -d 'Hardware type (rnode-lora, serial, tcp-client, tcp-server, auto)' -r
complete -c forge -n "__fish_forge_using_subcommand generate" -s n -l name -d 'Interface name (alphanumeric, hyphens, underscores)' -r
complete -c forge -n "__fish_forge_using_subcommand generate" -s f -l format -d 'Output format (reticulum, json, yaml)' -r
complete -c forge -n "__fish_forge_using_subcommand generate" -s o -l output -d 'Output file path (default: stdout)' -r -F
complete -c forge -n "__fish_forge_using_subcommand generate" -s P -l param -d 'Hardware-specific parameters as key=value pairs, e.g. -P freq=868mhz -P bw=125khz' -r
complete -c forge -n "__fish_forge_using_subcommand generate" -s v -l verbose -d 'Enable verbose logging'
complete -c forge -n "__fish_forge_using_subcommand generate" -s q -l quiet -d 'Suppress all output except errors'
complete -c forge -n "__fish_forge_using_subcommand generate" -s h -l help -d 'Print help'
complete -c forge -n "__fish_forge_using_subcommand simulate" -s n -l nodes -d 'Number of virtual nodes' -r
complete -c forge -n "__fish_forge_using_subcommand simulate" -s t -l topology -d 'Network topology (mesh, star, ring, chain)' -r
complete -c forge -n "__fish_forge_using_subcommand simulate" -s d -l duration -d 'Simulation duration (e.g. 30s, 5m, 1h)' -r
complete -c forge -n "__fish_forge_using_subcommand simulate" -s Q -l quality -d 'Link quality (excellent, good, moderate, poor)' -r
complete -c forge -n "__fish_forge_using_subcommand simulate" -s f -l format -d 'Output format (table, json, dot)' -r
complete -c forge -n "__fish_forge_using_subcommand simulate" -s o -l output -d 'Output file path (default: stdout)' -r -F
complete -c forge -n "__fish_forge_using_subcommand simulate" -s v -l verbose -d 'Enable verbose logging'
complete -c forge -n "__fish_forge_using_subcommand simulate" -s q -l quiet -d 'Suppress all output except errors'
complete -c forge -n "__fish_forge_using_subcommand simulate" -s h -l help -d 'Print help'
complete -c forge -n "__fish_forge_using_subcommand deploy" -s i -l inventory -d 'Inventory file path (nodes.toml)' -r
complete -c forge -n "__fish_forge_using_subcommand deploy" -s c -l concurrency -d 'Parallel deployment concurrency (max 32)' -r
complete -c forge -n "__fish_forge_using_subcommand deploy" -s t -l tag -d 'Tag filter — only deploy nodes with this tag' -r
complete -c forge -n "__fish_forge_using_subcommand deploy" -l config -d 'Config content to deploy (from file or stdin)' -r
complete -c forge -n "__fish_forge_using_subcommand deploy" -s f -l format -d 'Output format (table, json)' -r
complete -c forge -n "__fish_forge_using_subcommand deploy" -s o -l output -d 'Output file path (default: stdout)' -r -F
complete -c forge -n "__fish_forge_using_subcommand deploy" -l dry-run -d 'Dry run (show what would be deployed, no remote changes)'
complete -c forge -n "__fish_forge_using_subcommand deploy" -l provision -d 'Full provisioning (install Python, RNS, enable service)'
complete -c forge -n "__fish_forge_using_subcommand deploy" -s v -l verbose -d 'Enable verbose logging'
complete -c forge -n "__fish_forge_using_subcommand deploy" -s q -l quiet -d 'Suppress all output except errors'
complete -c forge -n "__fish_forge_using_subcommand deploy" -s h -l help -d 'Print help'
complete -c forge -n "__fish_forge_using_subcommand test" -l check -d 'Check type (connectivity, latency, redundancy, policies, all)' -r
complete -c forge -n "__fish_forge_using_subcommand test" -l threshold -d 'Latency threshold in milliseconds' -r
complete -c forge -n "__fish_forge_using_subcommand test" -s c -l config -d 'Config file path (forge.toml)' -r
complete -c forge -n "__fish_forge_using_subcommand test" -s f -l format -d 'Output format (table, json, tap, junit)' -r
complete -c forge -n "__fish_forge_using_subcommand test" -s o -l output -d 'Output file path (default: stdout)' -r -F
complete -c forge -n "__fish_forge_using_subcommand test" -s v -l verbose -d 'Enable verbose logging'
complete -c forge -n "__fish_forge_using_subcommand test" -s q -l quiet -d 'Suppress all output except errors'
complete -c forge -n "__fish_forge_using_subcommand test" -s h -l help -d 'Print help'
complete -c forge -n "__fish_forge_using_subcommand monitor" -s I -l inventory -d 'Inventory file path (nodes.toml)' -r
complete -c forge -n "__fish_forge_using_subcommand monitor" -s i -l interval -d 'Refresh interval in seconds' -r
complete -c forge -n "__fish_forge_using_subcommand monitor" -s v -l verbose -d 'Enable verbose logging'
complete -c forge -n "__fish_forge_using_subcommand monitor" -s q -l quiet -d 'Suppress all output except errors'
complete -c forge -n "__fish_forge_using_subcommand monitor" -s h -l help -d 'Print help'
complete -c forge -n "__fish_forge_using_subcommand completions" -s v -l verbose -d 'Enable verbose logging'
complete -c forge -n "__fish_forge_using_subcommand completions" -s q -l quiet -d 'Suppress all output except errors'
complete -c forge -n "__fish_forge_using_subcommand completions" -s h -l help -d 'Print help'
complete -c forge -n "__fish_forge_using_subcommand man" -s o -l output -d 'Output file path (default: stdout)' -r -F
complete -c forge -n "__fish_forge_using_subcommand man" -s v -l verbose -d 'Enable verbose logging'
complete -c forge -n "__fish_forge_using_subcommand man" -s q -l quiet -d 'Suppress all output except errors'
complete -c forge -n "__fish_forge_using_subcommand man" -s h -l help -d 'Print help'
complete -c forge -n "__fish_forge_using_subcommand help; and not __fish_seen_subcommand_from init generate simulate deploy test monitor completions man help" -f -a "init" -d 'Initialize a new Reticulum network project'
complete -c forge -n "__fish_forge_using_subcommand help; and not __fish_seen_subcommand_from init generate simulate deploy test monitor completions man help" -f -a "generate" -d 'Generate interface configs for hardware'
complete -c forge -n "__fish_forge_using_subcommand help; and not __fish_seen_subcommand_from init generate simulate deploy test monitor completions man help" -f -a "simulate" -d 'Simulate a virtual Reticulum network'
complete -c forge -n "__fish_forge_using_subcommand help; and not __fish_seen_subcommand_from init generate simulate deploy test monitor completions man help" -f -a "deploy" -d 'Deploy configs to remote nodes'
complete -c forge -n "__fish_forge_using_subcommand help; and not __fish_seen_subcommand_from init generate simulate deploy test monitor completions man help" -f -a "test" -d 'Test network config, connectivity, and policies'
complete -c forge -n "__fish_forge_using_subcommand help; and not __fish_seen_subcommand_from init generate simulate deploy test monitor completions man help" -f -a "monitor" -d 'Real-time network health dashboard (TUI)'
complete -c forge -n "__fish_forge_using_subcommand help; and not __fish_seen_subcommand_from init generate simulate deploy test monitor completions man help" -f -a "completions" -d 'Generate shell completions for bash, zsh, fish, or PowerShell'
complete -c forge -n "__fish_forge_using_subcommand help; and not __fish_seen_subcommand_from init generate simulate deploy test monitor completions man help" -f -a "man" -d 'Generate man page for forge'
complete -c forge -n "__fish_forge_using_subcommand help; and not __fish_seen_subcommand_from init generate simulate deploy test monitor completions man help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
