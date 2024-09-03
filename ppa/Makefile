all: nym-repo-setup.deb

REPO_DEB=nym-repo-setup.deb
REPO_DEB_SRC=nym-repo-setup

$(REPO_DEB):
	dpkg-deb --build $(REPO_DEB_SRC) $(REPO_DEB)

clean:
	rm $(REPO_DEB)
