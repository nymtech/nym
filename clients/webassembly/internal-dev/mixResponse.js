class MixResponse {
    constructor(response, opts) {
        this._originalResponse = response

        // this.url = opts.url
        this._opts = opts
    }


    get body() {
        return this._originalResponse.body
    }

    get bodyUsed() {
        return this._originalResponse.bodyUsed
    }

    get headers() {
        return this._originalResponse.headers
    }

    get ok() {
        return this._originalResponse.ok
    }

    get redirected() {
        return this._originalResponse.redirected
    }

    get status() {
        return this._originalResponse.status
    }

    get statusText() {
        return this._originalResponse.statusText
    }

    get type() {
        return this._opts.type
    }

    get url() {
        return this._originalResponse.url
    }

    async arrayBuffer() {
        return this._originalResponse.arrayBuffer()
    }

    async blob() {
        return this._originalResponse.blob()
    }

    clone() {
        return new MixResponse(this._originalResponse.clone(), this._opts)
    }

    async formData() {
        return this._originalResponse.formData()
    }

    async json() {
        return this._originalResponse.json()
    }

    async text() {
        return this._originalResponse.text()
    }
}