package tools

import (
	"github.com/steven0lisa/agent-libs/go"
)

func init() {
	agentlib.RegisterToolProvider(func(a *agentlib.Agent) {
		a.RegisterTool(&GrepTool{})
		a.RegisterTool(&GlobTool{})
	})
}
