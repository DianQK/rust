function iterate() {
    local good=`sed -n '1p' bisect_result`
    local bad=`sed -n '2p' bisect_result`
    echo $good
    echo $bad
    local result=$((bad - good))
    echo "good: $good, bad: $bad"
    if [ $result -eq 1 ]; then
        echo "done"
        exit 0
    else
        local next=$((good + (result / 2)))
        echo $next
        bash bisect.sh $next
        exit_code=$?
        case $exit_code in
            0)
                good=$next
                ;;
            1)
                bad=$next
                ;;
            *)
                echo "failed: $exit_code"
                exit 1
                ;;
        esac
    fi
    echo $good > bisect_result
    echo $bad >> bisect_result
}

while true; do
    iterate
    sleep 1
done
