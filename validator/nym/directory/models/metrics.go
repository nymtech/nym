package models

// MixMetric is a report from each mixnode detailing recent traffic.
// Useful for creating visualisations.
type MixMetric struct {
	PubKey   string          `json:"pubKey" binding:"required"`
	Sent     map[string]uint `json:"sent" binding:"required"`
	Received *uint           `json:"received" binding:"required"`
}

// PersistedMixMetric is a saved MixMetric with a timestamp recording when it
// was seen by the directory server. It can be used to build visualizations of
// mixnet traffic.
type PersistedMixMetric struct {
	MixMetric
	Timestamp int64 `json:"timestamp" binding:"required"`
}
