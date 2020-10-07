package fixtures

import "github.com/nymtech/nym-directory/models"

// MixStatusesList A list of mix statuses
func MixStatusesList() []models.PersistedMixStatus {
	booltrue := true
	m1 := models.PersistedMixStatus{
		MixStatus: models.MixStatus{
			IPVersion: "6",
			PubKey:    "pubkey1",
			Up:        &booltrue,
		},
		Timestamp: 123,
	}

	m2 := models.PersistedMixStatus{
		MixStatus: models.MixStatus{
			IPVersion: "6",
			PubKey:    "pubkey1",
			Up:        &booltrue,
		},
		Timestamp: 1234,
	}

	statuses := []models.PersistedMixStatus{m1, m2}
	return statuses
}

// XSSMixStatus ...
func XSSMixStatus() models.MixStatus {
	booltrue := true
	xss := models.MixStatus{
		IPVersion: "6",
		PubKey:    "pubkey2<script>alert('gotcha')</script>",
		Up:        &booltrue,
	}
	return xss
}

// GoodMixStatus ...
func GoodMixStatus() models.MixStatus {
	booltrue := true
	return models.MixStatus{
		IPVersion: "6",
		PubKey:    "pubkey2",
		Up:        &booltrue,
	}
}

// GoodPersistedMixStatus ...
func GoodPersistedMixStatus() models.PersistedMixStatus {
	return models.PersistedMixStatus{
		MixStatus: GoodMixStatus(),
		Timestamp: 1234,
	}
}

// MixStatusReport ...
func MixStatusReport() models.MixStatusReport {
	return models.MixStatusReport{
		PubKey:           "key1",
		MostRecentIPV4:   true,
		Last5MinutesIPV4: 100,
		LastHourIPV4:     100,
		LastDayIPV4:      100,
		LastWeekIPV4:     100,
		LastMonthIPV4:    100,
		MostRecentIPV6:   true,
		Last5MinutesIPV6: 100,
		LastHourIPV6:     100,
		LastDayIPV6:      100,
		LastWeekIPV6:     100,
		LastMonthIPV6:    100,
	}
}
