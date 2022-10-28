#! /bin/bash
#
# installer updater
#

get_latest_release() {
	curl --silent "https://api.github.com/repos/$1/releases/latest" | # Get latest release from GitHub api
  	grep '"tag_name":' |                                            # Get tag line
  	sed -E 's/.*"([^"]+)".*/\1/'                                    # Pluck JSON value
}

fail() {
	echo $@ >&2
	exit 1
}

os_name() {
	case "$(uname | tr '[:upper:]' '[:lower:]')" in
  		linux*)
	    		echo -n linux
    			;;
		darwin*)
			echo -n osx
    			;;
  		msys*)
    			echo -n windows
    			;;
  		*)
    			fail "system not supported"
    			;;
	esac
}

message() {
	echo $@
}

check_tool() {
	local TOOLBIN=$1
	local TOOLPATH=$(which "$TOOLBIN")
	if [ -z "$TOOLPATH" ]; then
		fail "$TOOLBIN not found"
	fi
}


install_runner() {
	message "-== GOLEM GWASM runner DEV Update ==-"

	check_tool curl
	check_tool awk

  local BINNAME=gwasm-runner
	local OS_NAME=$(os_name)
	local TAG=$(get_latest_release golemfactory/gwasm-runner)

	[ "$OS_NAME" != "windows" ] || check_tool 7z

	local DIST_NAME=gwasm-runner-$(os_name)-${TAG}
	local CURRENT_PATH=$(which ${BINNAME})
	local CP=cp

	if [ -n "${CURRENT_PATH}" ]; then
		message "gwasm-runner already found"
		message "current path:          ${CURRENT_PATH}"
		local VER=$($BINNAME -V 2>/dev/null| awk "NR == 1 && \$1 == \"$BINNAME\" { print \$2 }")
		message "current version:       ${VER:-unknown}"
		message "new version:           ${TAG}"

		read -p "override (y/N): " Q
		while [ "$Q" != "y" ] && [ "${Q:-n}" != "n" ]; do
			echo wrong answer \"$Q\"
			read -p "override (y/N): " Q
		done
		[ "${Q:-n}" = "n" ] && exit 0
		test -w "$CURRENT_PATH" || CP="sudo cp"
	else
    local BASE_PATH="$HOME/bin"
    test -d "$HOME/.cargo/bin" && BASE_PATH="$HOME/.cargo/bin"
		CURRENT_PATH="$BASE_PATH/$BINNAME"
    mkdir -f "$BASE_PATH"
		message "installing to $CURRENT_PATH"
	fi

	local UPDATE_WORK_DIR="$(mktemp -d)"
	trap "rm -rf $UPDATE_WORK_DIR" EXIT

	echo -n "download ${DIST_NAME}.tar.gz  "
	curl -sSL https://github.com/golemfactory/gwasm-runner/releases/download/${TAG}/${DIST_NAME}.tar.gz | tar xz -C "${UPDATE_WORK_DIR}" -f -
	echo " [ done ] "

#	"$UPDATE_WORK_DIR/$DIST_NAME/golemcli" _int complete bash > $UPDATE_WORK_DIR/golemcli-complete.sh
	echo -n "installing to $CURRENT_PATH   "
	$CP "$UPDATE_WORK_DIR/$DIST_NAME/$BINNAME" "$CURRENT_PATH"
	echo " [ done ] "

#	if test -d /etc/bash_completion.d/; then
#		read -p "install autocomplete definitions for bash (y/N): " Q
#		while [ "$Q" != "y" ] && [ "${Q:-n}" != "n" ]; do
#			echo wrong answer \"$Q\"
#			read -p "install autocomplete definitions for bash (y/N): " Q
#		done
#		[ "${Q:-n}" = "n" ] && exit 0
#		sudo cp "$UPDATE_WORK_DIR/golemcli-complete.sh" /etc/bash_completion.d/golemcli
#	fi
}

install_runner </dev/tty
