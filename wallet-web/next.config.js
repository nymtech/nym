// according to https://nextjs.org/docs/messages/webpack5 this should
// improve speed of subsequent `next build` calls due to better caching
module.exports = {
    future: {
        webpack5: true,
    },
}