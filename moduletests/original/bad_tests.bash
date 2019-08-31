[ -z $([ -z $var ]) ]
[ -z "$([ -z "$var" ])" ]

[ -n $([ -n $var ]) ]
[ -n "$([ -n "$var" ])" ]

[ pre"fix$([ pre"fix$var" = pre"fixyes" ])" = pre"fixyes" ]
[ pre"fix$([ pre"fix$var" != pre"fixyes" ])" != pre"fixyes" ]
