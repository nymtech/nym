all: inner.inc inner32.S

inner32.S: inner32.q copy-collector align
	qhasm-arm-m4 < inner32.q | ./copy-collector | ./align | sed 's/\<inner\>/CRYPTO_SHARED_NAMESPACE(inner)/' > inner32.S

inner32.q: inner.q inner32.top inner32.desc inner32.q.do
	sh inner32.q.do > inner32.q

inner.inc: inner.q inner.top inner.desc inner.c.do
	sh inner.c.do > inner.inc
