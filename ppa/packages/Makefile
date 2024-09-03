all: deb

REPO_DEB=nym-repo-setup.deb
REPO_SRC=nym-repo-setup

META_DEB=nym-vpn.deb
META_SRC=nym-vpn

ALL_DEB=$(REPO_DEB) $(META_DEB)

deb: $(ALL_DEB)

$(REPO_DEB):
	dpkg-deb --build $(REPO_SRC) $(REPO_DEB)
	dpkg-name -o $(REPO_DEB)

$(META_DEB):
	dpkg-deb --build $(META_SRC) $(META_DEB)
	dpkg-name -o $(META_DEB)

clean:
	rm $(ALL_DEB)
