yO3pBDvtebF-1X67-7lmjXcd3ljhRuwqVnCKQK8c
curl -X GET "https://api.cloudflare.com/client/v4/user/tokens/verify" \
     -H "Authorization: Bearer yO3pBDvtebF-1X67-7lmjXcd3ljhRuwqVnCKQK8c" \
     -H "Content-Type:application/json"
