package models

// CocoHostInfo comes from a coconut node telling us it's alive
type CocoHostInfo struct {
	HostInfo
	Type string `json:"type" binding:"required"`
}

// CocoPresence holds presence info for a coconut node.
type CocoPresence struct {
	CocoHostInfo
	LastSeen int64 `json:"lastSeen" binding:"required"`
}

// MixNodeID is a request to knock a node out of the regular topology and into
// the disallowed list. It's a temporary band-aid until we have staking and
// doesn't relate to anything else.
type MixNodeID struct {
	PubKey string
}

// HostInfo comes from a node telling us it's alive
type HostInfo struct {
	Host     string `json:"host"`
	PubKey   string `json:"pubKey" binding:"required"`
	Version  string `json:"version" binding:"required"`
	Location string `json:"location"`
}

// MixProviderHostInfo comes from a node telling us it's alive
type MixProviderHostInfo struct {
	ClientListener string `json:"clientListener"`
	MixnetListener string `json:"mixnetListener"`
	PubKey         string `json:"pubKey" binding:"required"`
	Version        string `json:"version" binding:"required"`
	Location       string `json:"location"`
}

// MixProviderPresence holds presence info for a mix provider node
type MixProviderPresence struct {
	MixProviderHostInfo
	LastSeen int64 `json:"lastSeen" binding:"required"`
}

// MixNodePresence holds presence info for a mixnode
type MixNodePresence struct {
	MixHostInfo
	// MixStatusReport
	LastSeen int64 `json:"lastSeen" binding:"required"`
}

// MixHostInfo comes from a node telling us it's alive
type MixHostInfo struct {
	HostInfo
	Layer uint `json:"layer" binding:"required"`
}

// Presence lets the server tell clients when a node was last seen
type Presence struct {
	HostInfo
	LastSeen int64 `json:"lastSeen" binding:"required"`
}

// GatewayHostInfo comes from a node telling us it's alive
type GatewayHostInfo struct {
	ClientListener string `json:"clientListener" binding:"required"`
	MixnetListener string `json:"mixnetListener" binding:"required"`
	IdentityKey    string `json:"identityKey" binding:"required"`
	SphinxKey      string `json:"sphinxKey" binding:"required"`
	Version        string `json:"version" binding:"required"`
	Location       string `json:"location"`
}

// GatewayPresence holds presence info for a gateway node
type GatewayPresence struct {
	GatewayHostInfo
	LastSeen int64 `json:"lastSeen" binding:"required"`
}

// Topology shows us the current state of the overall Nym network
type Topology struct {
	CocoNodes        []CocoPresence        `json:"cocoNodes"`
	MixNodes         []MixNodePresence     `json:"mixNodes"`
	MixProviderNodes []MixProviderPresence `json:"mixProviderNodes"`
	Gateways         []GatewayPresence     `json:"gatewayNodes"`
}
