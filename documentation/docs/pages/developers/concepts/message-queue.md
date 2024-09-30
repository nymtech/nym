
2) actually waiting on messages: `send()` **just puts the message in the queue of cover traffic** so dropping a client before its actually sent is something that is possible and should be avoided (see the troubleshooting example TODO LINK) for more on this.
