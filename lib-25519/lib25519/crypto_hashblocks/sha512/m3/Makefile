all: inner.inc inner32.S

inner32.S: inner32.q copy-collector align inner32.S.do
	sh inner32.S.do > inner32.S

inner32.q: inner.q inner32.top inner32.desc inner32.q.do
	sh inner32.q.do > inner32.q

inner.inc: inner.q inner.top inner.desc inner.c.do
	sh inner.c.do > inner.inc
