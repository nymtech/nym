package models

import (
	_ "github.com/jinzhu/gorm"
)

// MixStatus indicates whether a given node is up or down, as reported by a Nym monitor node.
// The 'Up' field is pretty annoying. Gin and other HTTP routers ignore incoming json "false" values,
// so making it a pointer works. This necessitates crapification of the Up-related code, as you can't
// do `*true` or `&true`, you need a variable to point to or dereference. This is why you'll see e.g.
// things like `booltrue := true`, `&booltrue` in the codebase. Maybe there's a more elegant way to
// acheive that which a bigger gopher could clean up.
type MixStatus struct {
	PubKey    string `json:"pubKey" binding:"required" gorm:"index"`
	IPVersion string `json:"ipVersion" binding:"required"`
	Up        *bool  `json:"up" binding:"required"`
}

// PersistedMixStatus is a saved MixStatus with a timestamp recording when it
// was seen by the directory server. It can be used to build visualizations of
// mixnode uptime.
type PersistedMixStatus struct {
	MixStatus
	Timestamp int64 `json:"timestamp" binding:"required"`
}

// MixStatusReport gives a quick view of mixnode uptime performance
type MixStatusReport struct {
	PubKey           string `json:"pubKey" binding:"required" gorm:"primaryKey;unique"`
	MostRecentIPV4   bool   `json:"mostRecentIPV4" binding:"required"`
	Last5MinutesIPV4 int    `json:"last5MinutesIPV4" binding:"required"`
	LastHourIPV4     int    `json:"lastHourIPV4" binding:"required"`
	LastDayIPV4      int    `json:"lastDayIPV4" binding:"required"`
	LastWeekIPV4     int    `json:"lastWeekIPV4" binding:"required"`
	LastMonthIPV4    int    `json:"lastMonthIPV4" binding:"required"`
	MostRecentIPV6   bool   `json:"mostRecentIPV6" binding:"required"`
	Last5MinutesIPV6 int    `json:"last5MinutesIPV6" binding:"required"`
	LastHourIPV6     int    `json:"lastHourIPV6" binding:"required"`
	LastDayIPV6      int    `json:"lastDayIPV6" binding:"required"`
	LastWeekIPV6     int    `json:"lastWeekIPV6" binding:"required"`
	LastMonthIPV6    int    `json:"lastMonthIPV6" binding:"required"`
}
