all: inner32.S

inner32.S: inner32.q copy-collector align inner32.S.do
	sh inner32.S.do > inner32.S

inner32.q: inner.q inner32.desc inner32.q.do inner32.top
	sh inner32.q.do > inner32.q
